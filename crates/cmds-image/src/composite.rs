use crate::helpers::{decode_image, detect_format, encode_image};
use crate::prelude::*;

pub const NAME: &str = "image_composite";
const DEFINITION: &str = flow_lib::node_definition!("image_composite.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub base: ImageInput,
    pub overlay: ImageInput,
    #[serde(default)]
    pub x: i64,
    #[serde(default)]
    pub y: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub image: bytes::Bytes,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let base_bytes = input.base.resolve(ctx.http()).await?;
    let overlay_bytes = input.overlay.resolve(ctx.http()).await?;
    let mut base = decode_image(&base_bytes)?;
    let overlay = decode_image(&overlay_bytes)?;
    let format = detect_format(&base_bytes)?;

    image::imageops::overlay(&mut base, &overlay, input.x, input.y);

    let bytes = encode_image(&base, format)?;

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
    async fn test_composite() {
        let output = run(
            <_>::default(),
            Input {
                base: test_png(10, 10).into(),
                overlay: test_png(3, 3).into(),
                x: 2,
                y: 2,
            },
        )
        .await
        .unwrap();
        let img = decode_image(&output.image).unwrap();
        // Base dimensions preserved
        assert_eq!(img.width(), 10);
        assert_eq!(img.height(), 10);
    }
}
