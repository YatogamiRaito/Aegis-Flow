use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

const PATH_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub fn encode_uri_path(path: &str) -> String {
    utf8_percent_encode(path, PATH_ENCODE_SET).to_string()
}

#[derive(Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_secs: u64,
}

pub fn generate_directory_listing(dir_path: &Path, uri_path: &str, as_json: bool) -> Result<(String, String), std::io::Error> {
    let mut entries = Vec::new();
    
    // Add parent dir link if not at root
    if uri_path != "/" && !uri_path.is_empty() {
        entries.push(DirEntry {
            name: "..".to_string(),
            is_dir: true,
            size: 0,
            mtime_secs: 0,
        });
    }

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();
        
        let mtime_secs = metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        entries.push(DirEntry {
            name,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            mtime_secs,
        });
    }

    // Sort: directories first, then alphabetical
    entries.sort_by(|a, b| {
        if a.name == ".." {
            return std::cmp::Ordering::Less;
        }
        if b.name == ".." {
            return std::cmp::Ordering::Greater;
        }
        b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name))
    });

    if as_json {
        let json = serde_json::to_string(&entries)?;
        Ok((json, "application/json".to_string()))
    } else {
        let html = generate_html(&entries, uri_path);
        Ok((html, "text/html; charset=utf-8".to_string()))
    }
}

fn generate_html(entries: &[DirEntry], uri_path: &str) -> String {
    let mut html = String::new();
    let display_path = html_escape::encode_text(uri_path);
    
    html.push_str(&format!(
        "<!DOCTYPE html><html><head><title>Index of {}</title>\
        <style>body{{font-family:monospace}} th{{text-align:left}} table{{width:100%}} td{{padding:2px 10px}}</style>\
        </head><body><h1>Index of {}</h1><hr><table>",
        display_path, display_path
    ));
    
    html.push_str("<tr><th>Name</th><th>Last Modified</th><th>Size</th></tr>");
    
    let base_href = if uri_path.ends_with('/') {
        uri_path.to_string()
    } else if uri_path.is_empty() {
         "/".to_string()
    } else {
         format!("{}/", uri_path)
    };

    for entry in entries {
        let href = if entry.name == ".." {
            "../".to_string()
        } else {
            let encoded_name = encode_uri_path(&entry.name);
            format!("{}{}{}", base_href, encoded_name, if entry.is_dir { "/" } else { "" })
        };
        
        let display_name = html_escape::encode_text(&entry.name);
        
        let size_str = if entry.is_dir && entry.name == ".." {
            "-".to_string()
        } else if entry.is_dir {
            "-".to_string()
        } else {
            entry.size.to_string()
        };

        let date_str = if entry.mtime_secs > 0 {
            use chrono::{DateTime, Utc};
            let dt = DateTime::from_timestamp(entry.mtime_secs as i64, 0).unwrap_or_default();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "-".to_string()
        };

        html.push_str(&format!(
            "<tr><td><a href=\"{}\">{}{}</a></td><td>{}</td><td>{}</td></tr>",
            href, display_name, if entry.is_dir && entry.name != ".." { "/" } else { "" }, date_str, size_str
        ));
    }
    
    html.push_str("</table><hr></body></html>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;

    #[test]
    fn test_html_directory_listing() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("test.txt")).unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let (html, mime) = generate_directory_listing(dir.path(), "/somedir/", false).unwrap();
        
        assert_eq!(mime, "text/html; charset=utf-8");
        assert!(html.contains("<title>Index of /somedir/</title>"));
        assert!(html.contains("<a href=\"../\">..</a>"));
        assert!(html.contains("<a href=\"/somedir/subdir/\">subdir/</a>"));
        assert!(html.contains("<a href=\"/somedir/test.txt\">test.txt</a>"));
    }

    #[test]
    fn test_json_directory_listing() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("test.txt")).unwrap();

        let (json, mime) = generate_directory_listing(dir.path(), "/somedir/", true).unwrap();
        
        assert_eq!(mime, "application/json");
        let parsed: Vec<DirEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0].name, "..");
        assert_eq!(parsed[1].name, "test.txt");
        assert!(!parsed[1].is_dir);
    }
}
