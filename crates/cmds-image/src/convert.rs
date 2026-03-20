use crate::helpers::{decode_image, encode_image, format_to_mime, parse_format};
use crate::prelude::*;

pub const NAME: &str = "image_convert";
const DEFINITION: &str = flow_lib::node_definition!("image_convert.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub image: ImageInput,
    pub format: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub image: bytes::Bytes,
    pub mime_type: String,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let image_bytes = input.image.resolve(ctx.http()).await?;
    let img = decode_image(&image_bytes)?;
    let target_format = parse_format(&input.format)?;
    let bytes = encode_image(&img, target_format)?;
    let mime_type = format_to_mime(target_format).to_owned();

    Ok(Output {
        image: bytes.into(),
        mime_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{detect_format, encode_image};

    fn test_png() -> bytes::Bytes {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            4,
            4,
            image::Rgba([255, 0, 0, 255]),
        ));
        encode_image(&img, image::ImageFormat::Png).unwrap().into()
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_convert_png_to_bmp() {
        let output = run(
            <_>::default(),
            Input {
                image: test_png().into(),
                format: "bmp".to_owned(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.mime_type, "image/bmp");
        let fmt = detect_format(&output.image).unwrap();
        assert_eq!(fmt, image::ImageFormat::Bmp);
    }

    #[tokio::test]
    async fn test_convert_unsupported() {
        let err = run(
            <_>::default(),
            Input {
                image: test_png().into(),
                format: "tiff".to_owned(),
            },
        )
        .await;
        assert!(err.is_err());
    }
}
