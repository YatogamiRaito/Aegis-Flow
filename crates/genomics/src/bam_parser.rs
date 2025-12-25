//! BAM Header Parser
//!
//! Parses BAM file headers for reference information.

use crate::{GenomicsError, Result};
use std::collections::HashMap;
use tracing::{debug, info};

/// BAM file header information
#[derive(Debug, Clone, Default)]
pub struct BamHeader {
    /// Version
    pub version: Option<String>,
    /// Sorting order
    pub sort_order: Option<String>,
    /// Reference sequences
    pub references: Vec<ReferenceSequence>,
    /// Read groups
    pub read_groups: Vec<ReadGroup>,
    /// Programs
    pub programs: Vec<Program>,
}

/// Reference sequence from BAM header
#[derive(Debug, Clone)]
pub struct ReferenceSequence {
    /// Sequence name
    pub name: String,
    /// Sequence length
    pub length: u64,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

/// Read group from BAM header
#[derive(Debug, Clone)]
pub struct ReadGroup {
    /// Read group ID
    pub id: String,
    /// Sample name
    pub sample: Option<String>,
    /// Library
    pub library: Option<String>,
    /// Platform
    pub platform: Option<String>,
}

/// Program record from BAM header
#[derive(Debug, Clone)]
pub struct Program {
    /// Program ID
    pub id: String,
    /// Program name
    pub name: Option<String>,
    /// Command line
    pub command_line: Option<String>,
    /// Version
    pub version: Option<String>,
}

impl BamHeader {
    /// Create a new empty header
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse header from SAM text format
    pub fn from_sam_text(text: &str) -> Result<Self> {
        let mut header = BamHeader::new();

        for line in text.lines() {
            if line.starts_with("@HD") {
                header.parse_hd_line(line)?;
            } else if line.starts_with("@SQ") {
                header.parse_sq_line(line)?;
            } else if line.starts_with("@RG") {
                header.parse_rg_line(line)?;
            } else if line.starts_with("@PG") {
                header.parse_pg_line(line)?;
            }
        }

        info!(
            "Parsed BAM header: {} references, {} read groups",
            header.references.len(),
            header.read_groups.len()
        );

        Ok(header)
    }

    fn parse_hd_line(&mut self, line: &str) -> Result<()> {
        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "VN" => self.version = Some(value.to_string()),
                    "SO" => self.sort_order = Some(value.to_string()),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn parse_sq_line(&mut self, line: &str) -> Result<()> {
        let mut name = String::new();
        let mut length: u64 = 0;
        let mut attributes = HashMap::new();

        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "SN" => name = value.to_string(),
                    "LN" => {
                        length = value.parse().map_err(|_| {
                            GenomicsError::ParseError("Invalid sequence length".to_string())
                        })?
                    }
                    _ => {
                        attributes.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        if !name.is_empty() {
            self.references.push(ReferenceSequence {
                name,
                length,
                attributes,
            });
            debug!(
                "Parsed reference: {} ({})",
                self.references.last().unwrap().name,
                length
            );
        }

        Ok(())
    }

    fn parse_rg_line(&mut self, line: &str) -> Result<()> {
        let mut id = String::new();
        let mut sample = None;
        let mut library = None;
        let mut platform = None;

        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "ID" => id = value.to_string(),
                    "SM" => sample = Some(value.to_string()),
                    "LB" => library = Some(value.to_string()),
                    "PL" => platform = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if !id.is_empty() {
            self.read_groups.push(ReadGroup {
                id,
                sample,
                library,
                platform,
            });
        }

        Ok(())
    }

    fn parse_pg_line(&mut self, line: &str) -> Result<()> {
        let mut id = String::new();
        let mut name = None;
        let mut command_line = None;
        let mut version = None;

        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "ID" => id = value.to_string(),
                    "PN" => name = Some(value.to_string()),
                    "CL" => command_line = Some(value.to_string()),
                    "VN" => version = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if !id.is_empty() {
            self.programs.push(Program {
                id,
                name,
                command_line,
                version,
            });
        }

        Ok(())
    }

    /// Get total reference length
    pub fn total_length(&self) -> u64 {
        self.references.iter().map(|r| r.length).sum()
    }

    /// Get reference by name
    pub fn get_reference(&self, name: &str) -> Option<&ReferenceSequence> {
        self.references.iter().find(|r| r.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sam_header() {
        let header_text = r"@HD	VN:1.6	SO:coordinate
@SQ	SN:chr1	LN:248956422
@SQ	SN:chr2	LN:242193529
@RG	ID:sample1	SM:NA12878	PL:ILLUMINA
@PG	ID:bwa	PN:bwa	VN:0.7.17	CL:bwa mem ref.fa reads.fq";

        let header = BamHeader::from_sam_text(header_text).unwrap();

        assert_eq!(header.version, Some("1.6".to_string()));
        assert_eq!(header.sort_order, Some("coordinate".to_string()));
        assert_eq!(header.references.len(), 2);
        assert_eq!(header.references[0].name, "chr1");
        assert_eq!(header.references[0].length, 248956422);
        assert_eq!(header.read_groups.len(), 1);
        assert_eq!(header.programs.len(), 1);
    }

    #[test]
    fn test_total_length() {
        let header_text = "@SQ	SN:chr1	LN:1000\n@SQ	SN:chr2	LN:2000";
        let header = BamHeader::from_sam_text(header_text).unwrap();

        assert_eq!(header.total_length(), 3000);
    }

    #[test]
    fn test_get_reference() {
        let header_text = "@SQ	SN:chr1	LN:1000\n@SQ	SN:chr2	LN:2000";
        let header = BamHeader::from_sam_text(header_text).unwrap();

        let chr1 = header.get_reference("chr1").unwrap();
        assert_eq!(chr1.length, 1000);

        assert!(header.get_reference("chr99").is_none());
    }
}
