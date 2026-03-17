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
    cmd!(sh, "./gen-secrets.ts --force").run()?;
    let tag = tag.map(Ok).unwrap_or_else(|| get_tag(sh))?;
    let pull = if compile { "missing" } else { "always" };
    let default_repo = if compile { "" } else { "public.ecr.aws/" };
    let flow_image = std::env::var("IMAGE")
        .unwrap_or_else(|_| format!("{default_repo}space-operator/flow-server:{tag}"));
    let cmds_image = std::env::var("CMDS_IMAGE")
        .unwrap_or_else(|_| format!("{default_repo}space-operator/cmds-server:{tag}"));
    cmd!(
        sh,
        "docker compose -f with-cmds-server.yml up --quiet-pull --pull {pull} -d --wait flow-server cmds-server deno-cmds-server webhook auth rest kong db"
    )
    .env("IMAGE", flow_image)
    .env("CMDS_IMAGE", cmds_image)
    .run()?;
    dotenv::from_path(meta.workspace_root.join("docker/.env"))?;
    dotenv::from_path(
        meta.workspace_root.join("@space-operator/client/integration_tests/.env"),
    )
    .ok();
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
        sh.change_dir(&meta.workspace_root);
        cmd!(sh, "deno -A @space-operator/client/utils/print_errors.ts")
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
        cmd!(sh, "docker compose -f with-cmds-server.yml ps")
            .run()
            .inspect_err(|error| eprint!("{error}"))
            .ok();

        for name in [
            "flow-server",
            "cmds-server",
            "deno-cmds-server",
            "realtime",
            "auth",
            "kong",
            "rest",
            "db",
            "storage",
            "meta",
            "webhook",
        ] {
            logs_service(&sh, name);
        }
    }

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
