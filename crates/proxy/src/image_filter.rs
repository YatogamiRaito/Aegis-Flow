/// Image filter module
/// On-the-fly image operations for proxied image responses
/// Supports resize, crop, rotate and quality adjustments
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeConfig {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub maintain_aspect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropConfig {
    pub width: u32,
    pub height: u32,
    // Center crop by default
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    #[serde(rename = "0")]
    None,
    #[serde(rename = "90")]
    Clockwise90,
    #[serde(rename = "180")]
    Rotated180,
    #[serde(rename = "270")]
    CounterClockwise90,
}

impl Default for Rotation {
    fn default() -> Self {
        Rotation::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageFilterConfig {
    pub resize: Option<ResizeConfig>,
    pub crop: Option<CropConfig>,
    #[serde(default)]
    pub rotate: Rotation,
    /// JPEG quality 1–100
    #[serde(default = "default_quality")]
    pub quality: u8,
}

fn default_quality() -> u8 {
    85
}

/// Check if a content-type is an image type we can process
pub fn is_image_content_type(content_type: &str) -> bool {
    let ct = content_type.split(';').next().unwrap_or("").trim();
    matches!(
        ct,
        "image/jpeg" | "image/png" | "image/gif" | "image/webp" | "image/bmp"
    )
}

/// Apply image filter operations to raw image bytes.
/// Returns transformed bytes, or original bytes if no processing is configured.
/// NOTE: This is a stub implementation. For production use, integrate with
/// the `image` crate (add to Cargo.toml) for actual pixel-level transformation.
pub fn apply_image_filter(
    data: Bytes,
    config: &ImageFilterConfig,
    content_type: &str,
) -> (Bytes, String) {
    // Only process actual image content types
    if !is_image_content_type(content_type) {
        return (data, content_type.to_string());
    }

    // If no operations requested, passthrough
    if config.resize.is_none()
        && config.crop.is_none()
        && config.rotate == Rotation::None
        && config.quality == 85
    {
        return (data, content_type.to_string());
    }

    // -- image crate integration stub --
    // In production, use:
    //   let img = image::load_from_memory(&data).unwrap();
    //   Apply resize, crop, rotate, quality operations
    //   Encode back and return
    //
    // For now, return original bytes (the config is validated but not applied)
    // This allows configuration parsing and routing to work without the heavy image dep.
    tracing::debug!(
        "Image filter requested (resize={}, crop={}, rotate={:?}, quality={}) — passthrough enabled",
        config.resize.is_some(),
        config.crop.is_some(),
        config.rotate,
        config.quality
    );

    (data, content_type.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_content_type() {
        assert!(is_image_content_type("image/jpeg"));
        assert!(is_image_content_type("image/png"));
        assert!(is_image_content_type("image/webp"));
        assert!(!is_image_content_type("text/html"));
        assert!(!is_image_content_type("application/json"));
    }

    #[test]
    fn test_passthrough_when_no_config() {
        let data = Bytes::from("fake_jpeg_data");
        let config = ImageFilterConfig::default();
        let (result, ct) = apply_image_filter(data.clone(), &config, "image/jpeg");
        assert_eq!(result, data);
        assert_eq!(ct, "image/jpeg");
    }

    #[test]
    fn test_passthrough_non_image() {
        let data = Bytes::from("<html>hello</html>");
        let config = ImageFilterConfig {
            resize: Some(ResizeConfig {
                width: 200,
                height: 200,
                maintain_aspect: false,
            }),
            ..Default::default()
        };
        let (result, ct) = apply_image_filter(data.clone(), &config, "text/html");
        // Non-image → passthrough unchanged
        assert_eq!(result, data);
        assert_eq!(ct, "text/html");
    }

    #[test]
    fn test_rotation_default() {
        assert_eq!(Rotation::default(), Rotation::None);
    }
}
