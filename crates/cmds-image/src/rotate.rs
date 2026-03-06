use crate::helpers::{decode_image, detect_format, encode_image};
use crate::prelude::*;

pub const NAME: &str = "image_rotate";
const DEFINITION: &str = flow_lib::node_definition!("image_rotate.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub image: ImageInput,
    #[serde(default)]
    pub angle: Option<String>,
    #[serde(default)]
    pub flip_h: Option<bool>,
    #[serde(default)]
    pub flip_v: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub image: bytes::Bytes,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let image_bytes = input.image.resolve(ctx.http()).await?;

    // Short-circuit if no operation requested
    if input.angle.is_none() && input.flip_h != Some(true) && input.flip_v != Some(true) {
        return Ok(Output {
            image: image_bytes,
        });
    }

    let mut img = decode_image(&image_bytes)?;
    let format = detect_format(&image_bytes)?;

    if let Some(angle) = &input.angle {
        img = match angle.as_str() {
            "90" => img.rotate90(),
            "180" => img.rotate180(),
            "270" => img.rotate270(),
            _ => return Err(CommandError::msg("angle must be 90, 180, or 270")),
        };
    }

    if input.flip_h == Some(true) {
        img = img.fliph();
    }
    if input.flip_v == Some(true) {
        img = img.flipv();
    }

    let bytes = encode_image(&img, format)?;

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
    async fn test_rotate_90() {
        let output = run(
            <_>::default(),
            Input {
                image: test_png(4, 2).into(),
                angle: Some("90".to_owned()),
                flip_h: None,
                flip_v: None,
            },
        )
        .await
        .unwrap();
        let img = decode_image(&output.image).unwrap();
        // 4x2 rotated 90° → 2x4
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 4);
    }

    #[tokio::test]
    async fn test_no_op_returns_input() {
        let png = test_png(4, 2);
        let output = run(
            <_>::default(),
            Input {
                image: png.clone().into(),
                angle: None,
                flip_h: None,
                flip_v: None,
            },
        )
        .await
        .unwrap();
        // Short-circuit: returns original bytes
        assert_eq!(output.image, png);
    }

    #[tokio::test]
    async fn test_invalid_angle() {
        let err = run(
            <_>::default(),
            Input {
                image: test_png(4, 2).into(),
                angle: Some("45".to_owned()),
                flip_h: None,
                flip_v: None,
            },
        )
        .await;
        assert!(err.is_err());
    }
}
