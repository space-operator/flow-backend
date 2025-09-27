use flow_lib::context::execute;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub enum TypedError {
    Unknown(#[serde_as(as = "flow_lib::errors::AsAnyhow")] anyhow::Error),
    Execute(execute::Error),
}

impl From<anyhow::Error> for TypedError {
    fn from(e: anyhow::Error) -> Self {
        let e = match e.downcast::<execute::Error>() {
            Ok(e) => return TypedError::Execute(e),
            Err(e) => e,
        };
        TypedError::Unknown(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_downcast() {
        let execute = execute::Error::Collected;
        let error = anyhow::Error::from(execute).context("run command");
        let typed = TypedError::from(error);
        assert!(matches!(typed, TypedError::Execute(_)));
    }
}
