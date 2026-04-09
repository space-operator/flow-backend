#![allow(clippy::print_stderr, clippy::print_stdout)]

use anyhow::{Context, Result};
use clap::Parser;
use std::io::Read;
use std::path::PathBuf;

/// Rust-native Groth16 prover for Umbra Privacy circuits.
///
/// Takes a pre-computed witness (from circom WASM) and a .zkey file,
/// generates a Groth16 proof, and outputs it as JSON.
///
/// The Bun node handles witness generation (running .wasm) and passes
/// the witness to this binary for the CPU-intensive proving step.
#[derive(Parser, Debug)]
#[command(name = "umbra-prover")]
struct Args {
    /// Path to the .zkey proving key file.
    #[arg(long)]
    zkey: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    eprintln!("[umbra-prover] zkey={}", args.zkey.display());

    // Read witness JSON from stdin (array of decimal strings)
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("read stdin")?;

    let witness_json: serde_json::Value =
        serde_json::from_str(&input).context("parse witness JSON")?;

    let witness =
        umbra_prover::prover::parse_witness_json(&witness_json).context("parse witness")?;

    eprintln!("[umbra-prover] witness: {} elements", witness.len());

    // Generate proof
    let proof = umbra_prover::prover::prove(&args.zkey, &witness).context("generate proof")?;

    // Serialize and output
    let output = umbra_prover::proof_format::to_json(&proof);
    let json = serde_json::to_string(&output).context("serialize output")?;
    println!("{json}");

    eprintln!("[umbra-prover] done");
    Ok(())
}
