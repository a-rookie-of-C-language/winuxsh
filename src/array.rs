// Array support for WinSH
use std::fmt;

/// Array value type
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayValue {
    String(String),
    Array(Vec<String>),
}

impl ArrayValue {
    /// Get the string value if this is a string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ArrayValue::String(s) => Some(s),
            _ => None,
        }
    }
}

impl fmt::Display for ArrayValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayValue::String(s) => write!(f, "{}", s),
            ArrayValue::Array(arr) => write!(f, "({})", arr.join(" ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_value() {
        let value = ArrayValue::String("hello".to_string());
        assert_eq!(value.as_string(), Some("hello"));
    }

    #[test]
    fn test_array_value() {
        let value = ArrayValue::Array(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(value.as_string(), None);
    }

    #[test]
    fn test_display() {
        let string_val = ArrayValue::String("hello".to_string());
        assert_eq!(string_val.to_string(), "hello");

        let array_val = ArrayValue::Array(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(array_val.to_string(), "(a b)");
    }
}
