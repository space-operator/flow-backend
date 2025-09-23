use flow_lib::context::execute;
use serde::Serialize;

#[derive(Debug)]
pub enum TypedError {
    Unknown(anyhow::Error),
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

impl Serialize for TypedError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TypedError::Unknown(error) => {
                serializer.serialize_newtype_variant("TypedError", 0, "Unknown", &error.to_string())
            }
            TypedError::Execute(error) => {
                serializer.serialize_newtype_variant("TypedError", 1, "Execute", &error)
            }
        }
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
