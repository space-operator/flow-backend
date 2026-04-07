#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::Path;
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

fn load_env_file_overriding(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }

    #[allow(deprecated)]
    for item in dotenv::from_path_iter(path)? {
        let (key, value) = item?;
        // The integration test runner mutates env vars before spawning child
        // processes so the Docker-local fixture config wins over repo-local .env.
        unsafe {
            std::env::set_var(key, value);
        }
    }

    Ok(())
}

fn run(sh: &Shell, compile: bool, tag: Option<String>) -> anyhow::Result<()> {
    let meta = cargo_metadata::MetadataCommand::new().no_deps().exec()?;
    sh.change_dir(&meta.workspace_root);

    if compile {
        cmd!(sh, "env PROFILE=dev ./scripts/build_images.bash docker").run()?;
    }

    sh.change_dir("docker/");
    // Tear down any leftover containers and volumes from previous runs to
    // guarantee a clean database.  Without this, stale wallets or flows from
    // a prior run (especially on self-hosted runners) can cause test failures.
    cmd!(sh, "docker compose -f with-cmds-server.yml down -v")
        .ignore_status()
        .run()?;
    cmd!(sh, "./gen-secrets.ts --force").run()?;
    let tag = tag.map(Ok).unwrap_or_else(|| get_tag(sh))?;
    let pull = if compile { "missing" } else { "always" };
    let ecr = "311141552572.dkr.ecr.us-west-2.amazonaws.com";
    let flow_image = std::env::var("IMAGE").unwrap_or_else(|_| {
        if compile {
            format!("flow-server:{tag}")
        } else {
            format!("{ecr}/flow-server:{tag}")
        }
    });
    let cmds_image = std::env::var("CMDS_IMAGE").unwrap_or_else(|_| {
        if compile {
            format!("cmds-server:{tag}")
        } else {
            format!("{ecr}/cmds-server:{tag}")
        }
    });
    cmd!(
        sh,
        "docker compose -f with-cmds-server.yml up --quiet-pull --pull {pull} -d --wait flow-server cmds-server deno-cmds-server webhook auth rest kong db"
    )
    .env("IMAGE", flow_image)
    .env("CMDS_IMAGE", cmds_image)
    .run()?;
    load_env_file_overriding(&meta.workspace_root.join("docker/.env"))?;
    load_env_file_overriding(
        &meta
            .workspace_root
            .join("@space-operator/client/integration_tests/.env"),
    )
    .ok();
    cmd!(
        sh,
        "deno run -A ./bootstrap-test-fixtures.ts --file=export.json --server=http://127.0.0.1:8080 --supabase-url=http://127.0.0.1:8000"
    )
    .run()?;
    load_env_file_overriding(&meta.workspace_root.join("docker/.flow-test.env")).ok();

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
        && let Ok(password) = cmd!(sh, "aws ecr get-login-password --region us-west-2")
            .read()
            .inspect_err(|error| eprint!("{error}"))
    {
        cmd!(
            sh,
            "docker login --username AWS --password-stdin 311141552572.dkr.ecr.us-west-2.amazonaws.com"
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
        cmd!(
            sh,
            "docker logout 311141552572.dkr.ecr.us-west-2.amazonaws.com"
        )
        .run()
        .inspect_err(|error| eprint!("{error}"))
        .ok();
    }

    if let Err(error) = result {
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}
