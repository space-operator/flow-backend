use flow_lib::command::prelude::*;

const NAME: &str = "chunk_string";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("string/chunk_string.json"))?
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
    let bytes = s.as_bytes();
    let len = bytes.len();

    let mut chunks = Vec::new();
    let mut pos = 0usize;

    while pos < len {
        let mut end = (pos + limit).min(len);

        if end == len {
            chunks.push(s[pos..end].to_string());
            break;
        }

        // Back up to a UTF-8 boundary if we're on a continuation byte.
        while end > pos && (bytes[end] & 0b1100_0000) == 0b1000_0000 {
            end -= 1;
        }

        if end == pos {
            // The next character is larger than the byte limit.
            return Err(CommandError::msg(
                "chunk_size is too small to contain a single UTF-8 character",
            ));
        }

        chunks.push(s[pos..end].to_string());
        pos = end;
    }

    Ok(Output { chunks })
}
