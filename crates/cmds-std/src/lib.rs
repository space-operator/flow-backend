#[macro_export]
macro_rules! node_definition {
    ($file:expr $(,)?) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/node-definitions/",
            $file
        ))
    };
}

pub mod const_cmd;
pub mod json_extract;
pub mod json_insert;
pub mod note;
pub mod print_cmd;
pub mod wait_cmd;
