RUST_LOG=info cargo run -p flow-server -- guest.toml
#[cfg(test)]
mod tests {
    use super::*;
}
git add .
git commit -m "Add unit tests for example functions"cargo test
