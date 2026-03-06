use crate::helpers::{decode_image, detect_format, encode_image};
use crate::prelude::*;

pub const NAME: &str = "image_crop";
const DEFINITION: &str = flow_lib::node_definition!("image_crop.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub image: ImageInput,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub image: bytes::Bytes,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    if input.width == 0 || input.height == 0 {
        return Err(CommandError::msg("crop width and height must be greater than 0"));
    }
    let image_bytes = input.image.resolve(ctx.http()).await?;
    let img = decode_image(&image_bytes)?;
    let format = detect_format(&image_bytes)?;
    let cropped = img.crop_imm(input.x, input.y, input.width, input.height);
    let bytes = encode_image(&cropped, format)?;

    Ok(Output {
        image: bytes.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{decode_image, encode_image};

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
    async fn test_crop() {
        let output = run(
            <_>::default(),
            Input {
                image: test_png(100, 100).into(),
                x: 10,
                y: 20,
                width: 30,
                height: 40,
            },
        )
        .await
        .unwrap();
        let img = decode_image(&output.image).unwrap();
        assert_eq!(img.width(), 30);
        assert_eq!(img.height(), 40);
    }

    #[tokio::test]
    async fn test_crop_zero_size() {
        let err = run(
            <_>::default(),
            Input {
                image: test_png(100, 100).into(),
                x: 0,
                y: 0,
                width: 0,
                height: 10,
            },
        )
        .await;
        assert!(err.is_err());
    }
}
