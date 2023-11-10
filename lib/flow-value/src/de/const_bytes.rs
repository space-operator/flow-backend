pub struct ConstBytes<const N: usize>(pub [u8; N]);

impl<'de, const N: usize> serde::Deserialize<'de> for ConstBytes<N> {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        d.deserialize_bytes(ConstBytesVisitor::<N>)
    }
}

struct ConstBytesVisitor<const N: usize>;

impl<'de, const N: usize> serde::de::Visitor<'de> for ConstBytesVisitor<N> {
    type Value = ConstBytes<N>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("bytes")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ConstBytes(<[u8; N]>::try_from(v).map_err(|_| {
            serde::de::Error::invalid_length(v.len(), &itoa::Buffer::new().format(N))
        })?))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let size = seq.size_hint();
        if let Some(size) = size {
            if size != N {
                return Err(serde::de::Error::invalid_length(
                    size,
                    &itoa::Buffer::new().format(N),
                ));
            }
        }

        let mut buf = [0u8; N];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = seq.next_element::<u8>()?.ok_or_else(|| {
                serde::de::Error::invalid_length(i, &itoa::Buffer::new().format(N))
            })?;
        }
        if seq.next_element::<u8>()?.is_some() {
            return Err(serde::de::Error::invalid_length(
                N + 1,
                &itoa::Buffer::new().format(N),
            ));
        }

        Ok(ConstBytes(buf))
    }
}
