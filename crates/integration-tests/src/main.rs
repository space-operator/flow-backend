#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::time::Duration;

use clap::Parser;
use xshell::{Shell, cmd};

fn get_tag(sh: &Shell) -> anyhow::Result<String> {
    let stdout = cmd!(sh, "git describe --always --dirty").read()?;
    let dirty = if stdout.trim().ends_with("-dirty") {
        "-dirty"
    } else {
        ""
    };

    let commit = cmd!(sh, "git rev-parse --verify HEAD").read()?;

    Ok(format!("{}{}", commit.trim(), dirty))
}

fn run(sh: &Shell, compile: bool, tag: Option<String>) -> anyhow::Result<()> {
    let meta = cargo_metadata::MetadataCommand::new().no_deps().exec()?;
    sh.change_dir(&meta.workspace_root);

    if compile {
        cmd!(sh, "env PROFILE=dev ./scripts/build_images.bash docker").run()?;
    }

    sh.change_dir("docker/");
    cmd!(sh, "./gen-secrets.ts").run()?;
    let tag = tag.map(Ok).unwrap_or_else(|| get_tag(sh))?;
    let repo = if compile { "" } else { "public.ecr.aws/" };
    cmd!(
        sh,
        "docker compose -f with-cmds-server.yml up --quiet-pull --pull always -d --wait"
    )
    .env("IMAGE", format!("{repo}space-operator/flow-server:{tag}"))
    .env(
        "CMDS_IMAGE",
        format!("{repo}space-operator/cmds-server:{tag}"),
    )
    .run()?;
    dotenv::from_path(meta.workspace_root.join("docker/.env"))?;
    cmd!(sh, "./import-data.ts --file=export.json").run()?;

    // wait for cmds-server to join
    std::thread::sleep(Duration::from_secs(5));

    sh.change_dir(&meta.workspace_root);
    sh.change_dir("@space-operator/client");

    cmd!(
        sh,
        "deno test --parallel -A --trace-leaks integration_tests"
    )
    .run()?;

    Ok(())
}

#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(long, action)]
    compile: bool,
    #[clap(long, action)]
    ecr_login: bool,
    #[clap(long)]
    tag: Option<String>,
}

fn main() {
    let mut args = Args::parse();

    let sh = Shell::new().unwrap();

    if args.ecr_login
        && let Ok(password) = cmd!(sh, "aws ecr-public get-login-password --region us-east-1")
            .read()
            .inspect_err(|error| eprint!("{error}"))
    {
        cmd!(
            sh,
            "docker login --username AWS --password-stdin public.ecr.aws/space-operator"
        )
        .stdin(password.trim())
        .run()
        .inspect_err(|error| {
            eprint!("{error}");
            args.ecr_login = false;
        })
        .ok();
    }

    dotenv::dotenv().ok();

    let meta = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .unwrap();

    let result = run(&sh, args.compile, args.tag);

    if result.is_err() {
        cmd!(sh, "deno -A utils/print_errors.ts")
            .run()
            .inspect_err(|error| eprint!("{error}"))
            .ok();
    }

    sh.change_dir(&meta.workspace_root);
    sh.change_dir("docker/");

    fn logs_service(sh: &Shell, name: &str) {
        cmd!(sh, "docker compose -f with-cmds-server.yml logs {name}")
            .run()
            .inspect_err(|error| eprint!("{error}"))
            .ok();
    }

    if result.is_err() {
        logs_service(&sh, "flow-server");
        logs_service(&sh, "cmds-server");
    }

    logs_service(&sh, "deno-cmds-server");
    logs_service(&sh, "webhook");

    cmd!(sh, "docker compose -f with-cmds-server.yml down -v")
        .ignore_stdout()
        .ignore_stderr()
        .run()
        .inspect_err(|error| eprint!("{error}"))
        .ok();
    cmd!(sh, "docker image prune -f")
        .run()
        .inspect_err(|error| eprint!("{error}"))
        .ok();

    if args.ecr_login {
        cmd!(sh, "docker logout public.ecr.aws/space-operator")
            .run()
            .inspect_err(|error| eprint!("{error}"))
            .ok();
    }

    if let Err(error) = result {
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}
