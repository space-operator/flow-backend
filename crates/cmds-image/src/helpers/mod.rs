pub mod image_input;

use flow_lib::command::CommandError;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

pub fn decode_image(bytes: &[u8]) -> Result<DynamicImage, CommandError> {
    image::load_from_memory(bytes).map_err(|e| CommandError::msg(format!("invalid image: {e}")))
}

pub fn encode_image(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>, CommandError> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), format)
        .map_err(|e| CommandError::msg(format!("failed to encode image: {e}")))?;
    Ok(buf)
}

pub fn parse_format(format: &str) -> Result<ImageFormat, CommandError> {
    match format.to_lowercase().as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpeg" | "jpg" => Ok(ImageFormat::Jpeg),
        "webp" => Ok(ImageFormat::WebP),
        "gif" => Ok(ImageFormat::Gif),
        "bmp" => Ok(ImageFormat::Bmp),
        other => Err(CommandError::msg(format!("unsupported format: {other}"))),
    }
}

pub fn format_to_mime(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Gif => "image/gif",
        ImageFormat::Bmp => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// Detect the format of an image from its bytes.
pub fn detect_format(bytes: &[u8]) -> Result<ImageFormat, CommandError> {
    image::guess_format(bytes)
        .map_err(|e| CommandError::msg(format!("cannot detect image format: {e}")))
}
