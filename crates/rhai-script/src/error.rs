use rhai::EvalAltResult;

/// Structured error from Rhai script evaluation, preserving line/column info.
#[derive(Debug, serde::Serialize)]
pub struct ScriptError {
    pub error_type: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, "[{}] line {}:{}: {}", self.error_type, line, col, self.message)
        } else if let Some(line) = self.line {
            write!(f, "[{}] line {}: {}", self.error_type, line, self.message)
        } else {
            write!(f, "[{}] {}", self.error_type, self.message)
        }
    }
}

impl std::error::Error for ScriptError {}

impl From<Box<EvalAltResult>> for ScriptError {
    fn from(err: Box<EvalAltResult>) -> Self {
        let pos = err.position();
        let error_type = match *err {
            EvalAltResult::ErrorParsing(..) => "ParseError",
            EvalAltResult::ErrorFunctionNotFound(..) => "FunctionNotFound",
            EvalAltResult::ErrorVariableNotFound(..) => "VariableNotFound",
            EvalAltResult::ErrorIndexNotFound(..) => "IndexNotFound",
            EvalAltResult::ErrorPropertyNotFound(..) => "PropertyNotFound",
            EvalAltResult::ErrorMismatchDataType(..) => "TypeMismatch",
            EvalAltResult::ErrorMismatchOutputType(..) => "OutputTypeMismatch",
            EvalAltResult::ErrorArithmetic(..) => "ArithmeticError",
            EvalAltResult::ErrorRuntime(..) => "RuntimeError",
            EvalAltResult::ErrorSystem(..) => "SystemError",
            EvalAltResult::ErrorTooManyOperations(..) => "TooManyOperations",
            EvalAltResult::ErrorTerminated(..) => "Terminated",
            _ => "ScriptError",
        }
        .to_owned();
        ScriptError {
            message: err.to_string(),
            error_type,
            line: pos.line(),
            column: pos.position(),
        }
    }
}
