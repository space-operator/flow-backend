use crate::helpers::{decode_image, detect_format, encode_image};
use crate::prelude::*;

pub const NAME: &str = "image_resize";
const DEFINITION: &str = flow_lib::node_definition!("image_resize.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub image: ImageInput,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default = "default_preserve")]
    pub preserve_aspect_ratio: bool,
}

fn default_preserve() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub image: bytes::Bytes,
    pub width: u32,
    pub height: u32,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    if input.width == 0 || input.height == 0 {
        return Err(CommandError::msg("width and height must be greater than 0"));
    }
    let image_bytes = input.image.resolve(ctx.http()).await?;
    let img = decode_image(&image_bytes)?;
    let format = detect_format(&image_bytes)?;

    let filter = match input.filter.as_deref() {
        Some("nearest") => image::imageops::FilterType::Nearest,
        Some("linear") => image::imageops::FilterType::Triangle,
        Some("cubic") => image::imageops::FilterType::CatmullRom,
        _ => image::imageops::FilterType::Lanczos3,
    };

    let resized = if input.preserve_aspect_ratio {
        img.resize(input.width, input.height, filter)
    } else {
        img.resize_exact(input.width, input.height, filter)
    };

    let width = resized.width();
    let height = resized.height();
    let bytes = encode_image(&resized, format)?;

    Ok(Output {
        image: bytes.into(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::encode_image;

    fn test_png(w: u32, h: u32) -> bytes::Bytes {
        let img = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255])),
        );
        encode_image(&img, image::ImageFormat::Png).unwrap().into()
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_resize() {
        let output = run(
            <_>::default(),
            Input {
                image: test_png(100, 200).into(),
                width: 50,
                height: 100,
                filter: None,
                preserve_aspect_ratio: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.width, 50);
        assert_eq!(output.height, 100);
    }

    #[tokio::test]
    async fn test_resize_preserve_aspect() {
        let output = run(
            <_>::default(),
            Input {
                image: test_png(100, 200).into(),
                width: 50,
                height: 200,
                filter: None,
                preserve_aspect_ratio: true,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.width, 50);
        assert_eq!(output.height, 100);
    }

    #[tokio::test]
    async fn test_resize_zero_width() {
        let err = run(
            <_>::default(),
            Input {
                image: test_png(10, 10).into(),
                width: 0,
                height: 10,
                filter: None,
                preserve_aspect_ratio: false,
            },
        )
        .await;
        assert!(err.is_err());
    }
}
