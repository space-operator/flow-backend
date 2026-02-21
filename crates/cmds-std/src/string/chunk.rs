use flow_lib::command::prelude::*;

const NAME: &str = "chunk_string";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("string/chunk_string.jsonc"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    string: String,
    chunk_size: u32,
}

#[derive(Serialize, Debug)]
struct Output {
    chunks: Vec<String>,
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    if input.chunk_size == 0 {
        return Err(CommandError::msg("chunk_size must be greater than 0"));
    }

    let limit = input.chunk_size as usize;
    let s = input.string;
    let len = s.len();

    let mut chunks = Vec::with_capacity(len.div_ceil(limit));
    let mut pos = 0;

    while pos < len {
        let mut end = (pos + limit).min(len);

        // Walk back to the nearest UTF-8 char boundary.
        while !s.is_char_boundary(end) {
            end -= 1;
        }

        if end == pos {
            return Err(CommandError::msg(
                "chunk_size is too small to contain a single UTF-8 character",
            ));
        }

        chunks.push(s[pos..end].to_string());
        pos = end;
    }

    Ok(Output { chunks })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_ascii() {
        let output = run(
            <_>::default(),
            Input {
                string: "abcdefghij".to_string(),
                chunk_size: 3,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.chunks, ["abc", "def", "ghi", "j"]);
    }

    #[tokio::test]
    async fn test_exact_fit() {
        let output = run(
            <_>::default(),
            Input {
                string: "abcdef".to_string(),
                chunk_size: 3,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.chunks, ["abc", "def"]);
    }

    #[tokio::test]
    async fn test_single_chunk() {
        let output = run(
            <_>::default(),
            Input {
                string: "hello".to_string(),
                chunk_size: 100,
            },
        )
        .await
        .unwrap();
        assert_eq!(output.chunks, ["hello"]);
    }

    #[tokio::test]
    async fn test_empty_string() {
        let output = run(
            <_>::default(),
            Input {
                string: String::new(),
                chunk_size: 5,
            },
        )
        .await
        .unwrap();
        assert!(output.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_utf8_boundary() {
        // 'é' is 2 bytes (U+00E9), chunk_size=3 means the second chunk
        // can't fit 'é' at the boundary and must walk back.
        let output = run(
            <_>::default(),
            Input {
                string: "aébc".to_string(), // bytes: [a(1), é(2), b(1), c(1)] = 5 bytes
                chunk_size: 3,
            },
        )
        .await
        .unwrap();
        // First chunk: "aé" (3 bytes), second chunk: "bc" (2 bytes)
        assert_eq!(output.chunks, ["aé", "bc"]);
    }

    #[tokio::test]
    async fn test_multibyte_emoji() {
        // '🦀' is 4 bytes (U+1F980)
        let output = run(
            <_>::default(),
            Input {
                string: "a🦀b".to_string(), // bytes: [a(1), 🦀(4), b(1)] = 6 bytes
                chunk_size: 5,
            },
        )
        .await
        .unwrap();
        // First chunk: "a🦀" (5 bytes), second chunk: "b" (1 byte)
        assert_eq!(output.chunks, ["a🦀", "b"]);
    }

    #[tokio::test]
    async fn test_chunk_size_zero() {
        let err = run(
            <_>::default(),
            Input {
                string: "hello".to_string(),
                chunk_size: 0,
            },
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("chunk_size must be greater than 0"));
    }

    #[tokio::test]
    async fn test_chunk_size_too_small_for_char() {
        // '🦀' is 4 bytes, chunk_size=2 can never fit it
        let err = run(
            <_>::default(),
            Input {
                string: "🦀".to_string(),
                chunk_size: 2,
            },
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("too small"));
    }
}
