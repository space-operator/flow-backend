use super::map_key::MapKeySerializer;
use super::seq::SerializeSeq;
use crate::{Error, Map, Value};

pub struct SerializeTupleVariant {
    pub name: &'static str,
    pub seq: SerializeSeq,
}

impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        use serde::ser::SerializeTuple;
        self.seq.serialize_element(value)?;
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        use serde::ser::SerializeTuple;
        let key = crate::Key::from(self.name);
        let value = self.seq.end()?;
        Ok(Value::Map(Map::from([(key, value)])))
    }
}

pub struct SerializeStructVariant {
    pub name: &'static str,
    pub map: Map,
}

impl serde::ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let value = value.serialize(super::Serializer)?;
        self.map.insert(crate::Key::from(key), value);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(Map::from([(
            crate::Key::from(self.name),
            Value::Map(self.map),
        )])))
    }
}

pub struct SerializeMap {
    pub map: Map,
    pub next_key: Option<crate::Key>,
}

impl serde::ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.next_key = Some(key.serialize(MapKeySerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let key = self
            .next_key
            .take()
            .expect("serialize_value called before serialize_key");
        let value = value.serialize(super::Serializer)?;
        self.map.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(self.map))
    }
}

impl serde::ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        serde::ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Value, Error> {
        serde::ser::SerializeMap::end(self)
    }
}
