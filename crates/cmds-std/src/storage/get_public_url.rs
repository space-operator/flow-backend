use super::FileSpec;
use flow_lib::command::prelude::*;

pub const NAME: &str = "storage_get_public_url";

const DEFINITION: &str = flow_lib::node_definition!("storage/get_public_url.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize)]
struct Output {
    url: String,
}

async fn run(ctx: CommandContextX, input: FileSpec) -> Result<Output, CommandError> {
    let key = input.key(&ctx.flow_owner().id);
    let url = format!(
        "{}/storage/v1/object/public/{}",
        ctx.endpoints().supabase,
        key
    );
    Ok(Output { url })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
