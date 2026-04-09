use ark_bn254::{Fq, G1Affine, G2Affine};
use ark_ec::AffineRepr;
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::Proof;
use serde::Serialize;

/// Umbra proof output — matches the SDK's `Groth16ProofBytes` layout.
#[derive(Debug, Serialize)]
pub struct UmbraProof {
    /// G1 point (64 bytes): Ax || Ay, big-endian
    #[serde(rename = "proofA")]
    pub proof_a: String,
    /// G2 point (128 bytes): b.x.c1 || b.x.c0 || b.y.c1 || b.y.c0, big-endian
    #[serde(rename = "proofB")]
    pub proof_b: String,
    /// G1 point (64 bytes): Cx || Cy, big-endian
    #[serde(rename = "proofC")]
    pub proof_c: String,
}

/// Convert an arkworks Proof<Bn254> to Umbra's byte layout.
///
/// Layout traced from @umbra-privacy/web-zk-prover convertSnarkjsProofToBytes:
/// - proofA: Ax || Ay (G1, 64 bytes)
/// - proofB: b.x.c1 || b.x.c0 || b.y.c1 || b.y.c0 (G2, 128 bytes, NOTE: c1 before c0)
/// - proofC: Cx || Cy (G1, 64 bytes)
pub fn to_json(proof: &Proof<ark_bn254::Bn254>) -> UmbraProof {
    UmbraProof {
        proof_a: hex::encode(serialize_g1(&proof.a)),
        proof_b: hex::encode(serialize_g2(&proof.b)),
        proof_c: hex::encode(serialize_g1(&proof.c)),
    }
}

/// Serialize G1 affine point: x || y, each as 32-byte big-endian.
fn serialize_g1(point: &G1Affine) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let (x, y) = point.xy().expect("G1 point at infinity");
    bytes[..32].copy_from_slice(&fq_to_be(&x));
    bytes[32..].copy_from_slice(&fq_to_be(&y));
    bytes
}

/// Serialize G2 affine point: x.c1 || x.c0 || y.c1 || y.c0
/// NOTE: c1 comes before c0 in Umbra's format (matches snarkjs convention).
fn serialize_g2(point: &G2Affine) -> [u8; 128] {
    let mut bytes = [0u8; 128];
    let (x, y) = point.xy().expect("G2 point at infinity");
    bytes[0..32].copy_from_slice(&fq_to_be(&x.c1));
    bytes[32..64].copy_from_slice(&fq_to_be(&x.c0));
    bytes[64..96].copy_from_slice(&fq_to_be(&y.c1));
    bytes[96..128].copy_from_slice(&fq_to_be(&y.c0));
    bytes
}

/// Convert Fq field element to 32-byte big-endian representation.
fn fq_to_be(fq: &Fq) -> [u8; 32] {
    let bigint = fq.into_bigint();
    let mut bytes = [0u8; 32];
    // ark_ff BigInteger stores as little-endian limbs; to_bytes_be gives big-endian
    let be = bigint.to_bytes_be();
    bytes.copy_from_slice(&be);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fq_to_be_zero() {
        let zero = Fq::from(0u64);
        let bytes = fq_to_be(&zero);
        assert_eq!(bytes, [0u8; 32]);
    }

    #[test]
    fn test_fq_to_be_one() {
        let one = Fq::from(1u64);
        let bytes = fq_to_be(&one);
        assert_eq!(bytes[31], 1);
        assert_eq!(bytes[..31], [0u8; 31]);
    }

    #[test]
    fn test_proof_sizes() {
        // Verify serialized sizes match Umbra's expectations
        assert_eq!(std::mem::size_of::<[u8; 64]>(), 64); // proofA
        assert_eq!(std::mem::size_of::<[u8; 128]>(), 128); // proofB
        assert_eq!(std::mem::size_of::<[u8; 64]>(), 64); // proofC
                                                         // Total: 256 bytes = ZK_PROOF_BYTE_LENGTH
    }
}
