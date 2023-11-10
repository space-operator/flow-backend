pub struct Array<I> {
    iter: I,
}

impl<I> Array<I> {
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I> serde::Serialize for Array<I>
where
    I: Iterator + Clone,
    I::Item: serde::Serialize,
{
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(None)?;
        for item in self.iter.clone() {
            seq.serialize_element(&item)?;
        }
        seq.end()
    }
}

pub struct Map<I> {
    iter: I,
}

impl<I> Map<I> {
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<'a, I, K, V> serde::Serialize for Map<I>
where
    I: Iterator<Item = (K, V)> + Clone,
    K: serde::Serialize + 'a,
    V: serde::Serialize + 'a,
{
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(None)?;
        for (k, v) in self.iter.clone() {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}
