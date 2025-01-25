#![allow(clippy::print_stderr, clippy::print_stdout)]

use xshell::{cmd, Shell};

fn get_tag(sh: &Shell) -> anyhow::Result<String> {
    let dirty = cmd!(sh, "git describe --always --dirty")
        .read()?
        .trim()
        .ends_with("-dirty")
        .then_some("-dirty")
        .unwrap_or("");

    let commit = cmd!(sh, "git rev-parse --verify HEAD").read()?;

    Ok(format!("{}{}", commit.trim(), dirty))
}

fn run(sh: &Shell) -> anyhow::Result<()> {
    let meta = cargo_metadata::MetadataCommand::new().no_deps().exec()?;
    sh.change_dir(&meta.workspace_root);

    cmd!(sh, "env PROFILE=dev ./scripts/build_images.bash docker").run()?;

    sh.change_dir("docker/");
    cmd!(sh, "./gen-secrets.ts").run()?;
    let tag = get_tag(sh)?;
    cmd!(
        sh,
        "env IMAGE=space-operator/flow-server:{tag} docker compose up -d --wait"
    )
    .run()?;
    dotenv::from_path(meta.workspace_root.join("docker/.env"))?;
    cmd!(sh, "./import-data.ts").run()?;

    sh.change_dir(&meta.workspace_root);
    sh.change_dir("@space-operator/client");

    cmd!(sh, "deno -A tests/auth.ts").run()?;
    cmd!(sh, "deno -A tests/deploy.ts").run()?;

    Ok(())
}

fn main() {
    let meta = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .unwrap();

    let sh = Shell::new().unwrap();
    dotenv::dotenv().ok();
    let result = run(&sh);

    sh.change_dir(&meta.workspace_root);
    sh.change_dir("docker/");
    cmd!(sh, "docker compose logs flow-server").run().ok();
    cmd!(sh, "docker compose down -v").run().ok();
    cmd!(sh, "docker compose down -v").run().ok();
    cmd!(sh, "docker image prune -f").run().ok();

    if let Err(error) = result {
        eprintln!("{:?}", error);
        std::process::exit(1);
    }
}
