use crate::with::AsPubkey;
use solana_pubkey::Pubkey;

type Target = Pubkey;

type As = AsPubkey;

pub mod opt {
    use serde_with::{DeserializeAs, SerializeAs};

    pub fn serialize<S>(sig: &Option<super::Target>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Option::<super::As>::serialize_as(sig, s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<super::Target>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<super::As>::deserialize_as(d)
    }
}

pub fn serialize<S>(p: &Target, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    As::serialize(p, s)
}

pub fn deserialize<'de, D>(d: D) -> Result<Target, D::Error>
where
    D: serde::Deserializer<'de>,
{
    As::deserialize(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use solana_keypair::Keypair;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;

    fn de<'de, D: serde::Deserializer<'de>>(d: D) -> Pubkey {
        deserialize(d).unwrap()
    }

    #[test]
    fn test_deserialize_value() {
        let id = solana_pubkey::new_rand();
        assert_eq!(de(Value::String(id.to_string())), id);
        assert_eq!(de(Value::B32(id.to_bytes())), id);

        let k = Keypair::new();
        let pk = k.pubkey();
        assert_eq!(de(Value::B64(k.to_bytes())), pk);
    }

    #[test]
    fn test_serialize() {
        let id = solana_pubkey::new_rand();
        assert_eq!(
            serialize(&id, crate::ser::Serializer).unwrap(),
            Value::B32(id.to_bytes())
        );
    }
}
