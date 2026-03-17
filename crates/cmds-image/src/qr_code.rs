use crate::helpers::encode_image;
use crate::prelude::*;

pub const NAME: &str = "qr_code";
const DEFINITION: &str = flow_lib::node_definition!("qr_code.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub data: String,
    #[serde(default = "default_size")]
    pub size: u32,
    #[serde(default)]
    pub error_correction: Option<String>,
}

fn default_size() -> u32 {
    256
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub image: bytes::Bytes,
    pub svg: Option<String>,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let ec = match input.error_correction.as_deref() {
        Some("low") => qrcode::EcLevel::L,
        Some("quartile") => qrcode::EcLevel::Q,
        Some("high") => qrcode::EcLevel::H,
        _ => qrcode::EcLevel::M,
    };

    let code = qrcode::QrCode::with_error_correction_level(&input.data, ec)
        .map_err(|e| CommandError::msg(format!("QR code generation failed: {e}")))?;

    // Generate SVG
    let svg = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(input.size, input.size)
        .build();

    // Generate PNG image
    let img = code
        .render::<image::Luma<u8>>()
        .min_dimensions(input.size, input.size)
        .build();

    let dynamic = image::DynamicImage::ImageLuma8(img);
    let bytes = encode_image(&dynamic, image::ImageFormat::Png)?;

    Ok(Output {
        image: bytes.into(),
        svg: Some(svg),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::decode_image;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_qr_code() {
        let output = run(
            <_>::default(),
            Input {
                data: "https://example.com".to_owned(),
                size: 128,
                error_correction: None,
            },
        )
        .await
        .unwrap();
        // Should produce a valid PNG
        let img = decode_image(&output.image).unwrap();
        assert!(img.width() >= 128);
        assert!(img.height() >= 128);
        // Should produce SVG
        assert!(output.svg.unwrap().contains("<svg"));
    }

    #[tokio::test]
    async fn test_qr_code_high_ec() {
        let output = run(
            <_>::default(),
            Input {
                data: "test".to_owned(),
                size: 64,
                error_correction: Some("high".to_owned()),
            },
        )
        .await
        .unwrap();
        assert!(!output.image.is_empty());
    }
}
