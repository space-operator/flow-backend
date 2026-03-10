use crate::helpers::{decode_image, detect_format, encode_image};
use crate::prelude::*;

pub const NAME: &str = "image_thumbnail";
const DEFINITION: &str = flow_lib::node_definition!("image_thumbnail.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub image: ImageInput,
    #[serde(default = "default_max")]
    pub max_dimension: u32,
}

fn default_max() -> u32 {
    128
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub image: bytes::Bytes,
    pub width: u32,
    pub height: u32,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    if input.max_dimension == 0 {
        return Err(CommandError::msg("max_dimension must be greater than 0"));
    }
    let image_bytes = input.image.resolve(ctx.http()).await?;
    let img = decode_image(&image_bytes)?;
    let format = detect_format(&image_bytes)?;
    let thumb = img.thumbnail(input.max_dimension, input.max_dimension);
    let width = thumb.width();
    let height = thumb.height();
    let bytes = encode_image(&thumb, format)?;

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
    async fn test_thumbnail() {
        let output = run(
            <_>::default(),
            Input {
                image: test_png(200, 100).into(),
                max_dimension: 50,
            },
        )
        .await
        .unwrap();
        // 200x100 → max 50 → 50x25
        assert_eq!(output.width, 50);
        assert_eq!(output.height, 25);
    }

    #[tokio::test]
    async fn test_thumbnail_zero() {
        let err = run(
            <_>::default(),
            Input {
                image: test_png(10, 10).into(),
                max_dimension: 0,
            },
        )
        .await;
        assert!(err.is_err());
    }
}
