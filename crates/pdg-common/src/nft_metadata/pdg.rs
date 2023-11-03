use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Attr<T> {
    #[serde(flatten)]
    pub cfg: AttrCfg,
    pub value: T,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttrCfg {
    pub concat: bool,
    pub flag: u8,
    pub own: bool,
    pub r#type: u8,
}

impl AttrCfg {
    pub const fn new_type(ty: u8) -> Self {
        Self {
            concat: false,
            flag: 0,
            own: false,
            r#type: ty,
        }
    }
}
