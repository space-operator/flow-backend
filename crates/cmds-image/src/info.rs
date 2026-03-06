use crate::helpers::{decode_image, detect_format};
use crate::prelude::*;

pub const NAME: &str = "image_info";
const DEFINITION: &str = flow_lib::node_definition!("image_info.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub image: ImageInput,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub color_type: String,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let image_bytes = input.image.resolve(ctx.http()).await?;
    let img = decode_image(&image_bytes)?;
    let fmt = detect_format(&image_bytes)?;

    Ok(Output {
        width: img.width(),
        height: img.height(),
        format: format!("{:?}", fmt).to_lowercase(),
        color_type: format!("{:?}", img.color()),
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
    async fn test_run() {
        let output = run(<_>::default(), Input { image: test_png(4, 3).into() })
            .await
            .unwrap();
        assert_eq!(output.width, 4);
        assert_eq!(output.height, 3);
        assert_eq!(output.format, "png");
    }
}
