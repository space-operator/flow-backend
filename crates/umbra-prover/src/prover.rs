use anyhow::{Context, Result};
use ark_bn254::Bn254;
use ark_ff::PrimeField;
use ark_groth16::Groth16;
use ark_std::UniformRand;
use circom_prover::prover::ark_circom::{read_zkey, CircomReduction, ZkeyHeaderReader};
use num_bigint::BigUint;
use std::io::BufReader;
use std::path::Path;

/// Generate a Groth16 proof from a pre-computed witness and zkey file.
pub fn prove(zkey_path: &Path, witness: &[BigUint]) -> Result<ark_groth16::Proof<Bn254>> {
    // Verify this is a BN254 zkey
    eprintln!("[prover] checking zkey header...");
    let mut header = ZkeyHeaderReader::new(zkey_path.to_str().context("zkey path")?);
    header.read();
    let expected_prime = BigUint::from(ark_bn254::Fr::MODULUS);
    if header.r != expected_prime {
        anyhow::bail!("zkey is not BN254");
    }

    // Load proving key + constraint matrices
    eprintln!("[prover] loading proving key...");
    let file = std::fs::File::open(zkey_path).context("open zkey")?;
    let mut reader = BufReader::new(file);
    let (pk, matrices) = read_zkey::<_, Bn254>(&mut reader).context("read zkey")?;

    eprintln!(
        "[prover] {} constraints, {} instance vars",
        matrices.num_constraints, matrices.num_instance_variables
    );

    // Convert witness to field elements
    let witness_fr: Vec<ark_bn254::Fr> = witness
        .iter()
        .map(|v| ark_bn254::Fr::from(v.clone()))
        .collect();

    eprintln!("[prover] witness: {} elements", witness_fr.len());

    // Generate proof using Groth16 with CircomReduction
    eprintln!("[prover] generating Groth16 proof...");
    let mut rng = ark_std::rand::thread_rng();
    let r = ark_bn254::Fr::rand(&mut rng);
    let s = ark_bn254::Fr::rand(&mut rng);

    let proof = Groth16::<Bn254, CircomReduction>::create_proof_with_reduction_and_matrices(
        &pk,
        r,
        s,
        &matrices,
        matrices.num_instance_variables,
        matrices.num_constraints,
        witness_fr.as_slice(),
    )
    .context("Groth16 prove")?;

    eprintln!("[prover] proof generated successfully");
    Ok(proof)
}

/// Parse a witness from JSON (array of decimal strings).
pub fn parse_witness_json(json: &serde_json::Value) -> Result<Vec<BigUint>> {
    let arr = json.as_array().context("witness must be a JSON array")?;
    arr.iter()
        .enumerate()
        .map(|(i, v)| {
            let s = v
                .as_str()
                .context(format!("witness[{i}] must be a string"))?;
            s.parse::<BigUint>()
                .context(format!("witness[{i}]: invalid BigUint"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_witness_json() {
        let json = serde_json::json!(["1", "42", "0"]);
        let witness = parse_witness_json(&json).unwrap();
        assert_eq!(witness.len(), 3);
        assert_eq!(witness[0], BigUint::from(1u64));
        assert_eq!(witness[1], BigUint::from(42u64));
        assert_eq!(witness[2], BigUint::from(0u64));
    }
}
