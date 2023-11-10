use std::borrow::Cow;
use thiserror::Error as ThisError;

/// [JSON Pointer](https://www.rfc-editor.org/rfc/rfc6901)
pub struct Path<'a> {
    pub segments: Vec<Cow<'a, str>>,
}

#[derive(ThisError, Debug)]
#[error("invalid path")]
pub struct InvalidPath;

impl<'a> Path<'a> {
    pub fn parse(s: &str) -> Result<Path<'_>, InvalidPath> {
        parse(s)
    }

    pub fn to_owned(self) -> Path<'static> {
        Path {
            segments: self
                .segments
                .into_iter()
                .map(|s: Cow<'a, str>| -> Cow<'static, str> {
                    Cow::Owned(match s {
                        Cow::Borrowed(b) => b.to_owned(),
                        Cow::Owned(o) => o,
                    })
                })
                .collect(),
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Cow<str>> {
        self.segments.iter()
    }
}

fn parse(mut s: &str) -> Result<Path<'_>, InvalidPath> {
    if s.is_empty() {
        return Ok(Path {
            segments: Vec::new(),
        });
    }

    if s.starts_with('/') {
        s = &s[1..];
    }

    let mut vec = Vec::new();

    loop {
        let end = s.find('/').unwrap_or(s.len());
        let segment = &s[..end];
        if segment.contains('~') {
            // validate escape characters
            let buf = segment.as_bytes();
            if *buf
                .last()
                .expect("segment.contains('~') so it is not empty")
                == b'~'
            {
                return Err(InvalidPath);
            }
            for w in buf.windows(2) {
                if w[0] == b'~' && w[1] != b'0' && w[1] != b'1' {
                    return Err(InvalidPath);
                }
            }

            let mut segment = segment.replace("~1", "/");
            segment = segment.replace("~0", "~");
            vec.push(Cow::Owned(segment));
        } else {
            vec.push(Cow::Borrowed(segment));
        }

        if end == s.len() {
            break;
        } else {
            s = &s[(end + 1)..];
        }
    }

    Ok(Path { segments: vec })
}

impl<'a> IntoIterator for Path<'a> {
    type Item = <Vec<Cow<'a, str>> as IntoIterator>::Item;

    type IntoIter = <Vec<Cow<'a, str>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.into_iter()
    }
}
