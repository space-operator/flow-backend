use crate::{Error, Value};
use serde::de::{value::U32Deserializer, IntoDeserializer};

pub struct EnumDeserializer {
    pub variant: String,
    pub value: Option<Value>,
}

impl<'de> serde::de::EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer), Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let visitor = VariantDeserializer { value: self.value };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

pub struct VariantDeserializer {
    pub value: Option<Value>,
}

impl<'de> serde::de::VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => serde::Deserialize::deserialize(value),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Some(Value::Array(v)) => {
                if v.is_empty() {
                    visitor.visit_unit()
                } else {
                    super::visit_array(v, visitor)
                }
            }
            Some(other) => Err(serde::de::Error::invalid_type(
                other.unexpected(),
                &"tuple variant",
            )),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Some(Value::Map(v)) => super::visit_map(v, visitor),
            Some(other) => Err(serde::de::Error::invalid_type(
                other.unexpected(),
                &"struct variant",
            )),
            None => Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

pub struct ValueEnumAccess(pub Value);

impl<'de> serde::de::EnumAccess<'de> for ValueEnumAccess {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let kind = U32Deserializer::<Error>::new(self.0.kind() as u32);
        let variant = seed.deserialize(kind)?;
        Ok((variant, self))
    }
}

impl<'de> serde::de::VariantAccess<'de> for ValueEnumAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        todo!()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.0)
    }

    fn tuple_variant<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(serde::de::Error::invalid_type(
            serde::de::Unexpected::TupleVariant,
            &"newtype variant",
        ))
    }

    fn struct_variant<V>(self, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(serde::de::Error::invalid_type(
            serde::de::Unexpected::StructVariant,
            &"newtype variant",
        ))
    }
}
