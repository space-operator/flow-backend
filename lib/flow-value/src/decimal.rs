use rust_decimal::Decimal;

pub(crate) const TOKEN: &str = "$$d";

pub type Target = Decimal;

pub mod opt {
    pub fn serialize<S>(sig: &Option<super::Target>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match sig {
            Some(sig) => super::serialize(sig, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<super::Target>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_option(crate::OptionVisitor(super::Visitor))
    }
}

pub fn serialize<S>(d: &Decimal, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_newtype_struct(TOKEN, &crate::Bytes(&d.serialize()))
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = Decimal;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("decimal")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v.len() != 16 {
            return Err(serde::de::Error::invalid_length(v.len(), &"16"));
        }

        let buf: [u8; 16] = v.try_into().unwrap();
        Ok(Decimal::deserialize(buf))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Decimal::from(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Decimal::from(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // TODO: this is lossy
        Decimal::try_from(v).map_err(serde::de::Error::custom)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let v = v.trim();
        if v.bytes().any(|c| c == b'e' || c == b'E') {
            Decimal::from_scientific(v).map_err(serde::de::Error::custom)
        } else {
            v.parse().map_err(serde::de::Error::custom)
        }
    }

    fn visit_newtype_struct<D>(self, d: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_any(self)
    }
}

pub fn deserialize<'de, D>(d: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_newtype_struct(TOKEN, Visitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use rust_decimal_macros::dec;

    fn de<'de, D: serde::Deserializer<'de>>(d: D) -> Decimal {
        deserialize(d).unwrap()
    }

    #[test]
    fn test_deserialize_value() {
        assert_eq!(de(Value::U64(100)), dec!(100));
        assert_eq!(de(Value::I64(-1)), dec!(-1));
        assert_eq!(de(Value::Decimal(Decimal::MAX)), Decimal::MAX);
        assert_eq!(de(Value::F64(1231.2221)), dec!(1231.2221));
        assert_eq!(de(Value::String("1234.0".to_owned())), dec!(1234));
        assert_eq!(de(Value::String("  1234.0".to_owned())), dec!(1234));
        assert_eq!(de(Value::String("1e5".to_owned())), dec!(100000));
        assert_eq!(de(Value::String("  1e5".to_owned())), dec!(100000));
    }

    #[test]
    fn test_serialize() {
        assert_eq!(
            serialize(&Decimal::MAX, crate::ser::Serializer).unwrap(),
            Value::Decimal(Decimal::MAX)
        );
    }
}
