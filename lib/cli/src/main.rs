use std::io::stdin;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Login to Space Operator using API key
    Login {},
}

fn main() {
    let args = Args::parse();
    match &args.command {
        Some(Commands::Login {}) => {
            println!("Go to https://spaceoperator.com/dashboard/profile/apikey go generate a key");
            println!("Please paste your API key below");
            let mut key = String::new();
            stdin().read_line(&mut key).unwrap();
        }
        None => {}
    }
}
