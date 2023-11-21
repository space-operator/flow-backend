use flow_lib::{
    command::{
        builder::{BuildResult, BuilderCache, CmdBuilder},
        CommandDescription, CommandError,
    },
    Context,
};
use pdg_common::nft_metadata::RenderParams;
use serde::{Deserialize, Serialize};
use tracing::info;

const GEN_PDG_ATTRS: &str = "gen_pdg_attrs";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("gen_pdg_attrs.json"))?.check_name(GEN_PDG_ATTRS)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(GEN_PDG_ATTRS, |_| build()));

const fn bool_true() -> bool {
    true
}

#[derive(Deserialize, Debug)]
struct Input {
    attributes: Option<RenderParams>,
    #[serde(default = "bool_true")]
    gen_human_readable: bool,
    flag: Option<String>,
}

#[derive(Serialize, Debug)]
struct Output {
    attributes: serde_json::Value,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    let attributes = match input.flag {
        Some(flag) => match flag.as_str() {
            "base" => RenderParams::generate_base(),
            _ => RenderParams::default(),
        },
        None => input.attributes.unwrap_or_default(),
    }
    .to_pdg_metadata(input.gen_human_readable)?;

    info!("{:#?}", attributes);

    Ok(Output { attributes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::value;

    #[tokio::test]
    async fn test_generate() {
        let output = build()
            .unwrap()
            .run(
                <_>::default(),
                value::map! {
                    "flag" => "base",
                },
            )
            .await
            .unwrap();
        let attrs = &output["attributes"];
        let pose = value::crud::get(attrs, &["Pose", "value"]).unwrap();
        assert!(matches!(pose, flow_lib::Value::Array(_)));
    }
}
