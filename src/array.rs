// Array support for WinSH
use std::fmt;

/// Array value type
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayValue {
    String(String),
    Array(Vec<String>),
}

impl ArrayValue {
    /// Create a new string value
    pub fn string(s: impl Into<String>) -> Self {
        ArrayValue::String(s.into())
    }

    /// Create a new array value
    pub fn array(elements: Vec<String>) -> Self {
        ArrayValue::Array(elements)
    }

    /// Get the string value if this is a string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ArrayValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get the array value if this is an array
    pub fn as_array(&self) -> Option<&[String]> {
        match self {
            ArrayValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get the length of the value
    pub fn len(&self) -> usize {
        match self {
            ArrayValue::String(s) => s.len(),
            ArrayValue::Array(arr) => arr.len(),
        }
    }

    /// Check if the value is empty
    pub fn is_empty(&self) -> bool {
        match self {
            ArrayValue::String(s) => s.is_empty(),
            ArrayValue::Array(arr) => arr.is_empty(),
        }
    }

    /// Get an element by index (for arrays, returns first element for strings)
    pub fn get(&self, index: usize) -> Option<&str> {
        match self {
            ArrayValue::String(s) => {
                if index == 0 {
                    Some(s)
                } else {
                    None
                }
            }
            ArrayValue::Array(arr) => arr.get(index).map(|s| s.as_str()),
        }
    }

    /// Get all elements (for arrays, returns vector with string for strings)
    pub fn all(&self) -> Vec<&str> {
        match self {
            ArrayValue::String(s) => vec![s.as_str()],
            ArrayValue::Array(arr) => arr.iter().map(|s| s.as_str()).collect(),
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
        let value = ArrayValue::string("hello");
        assert_eq!(value.as_string(), Some("hello"));
        assert_eq!(value.as_array(), None);
        assert_eq!(value.get(0), Some("hello"));
        assert_eq!(value.get(1), None);
    }

    #[test]
    fn test_array_value() {
        let value = ArrayValue::array(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(value.as_string(), None);
        assert_eq!(
            value.as_array(),
            Some(&["a".to_string(), "b".to_string(), "c".to_string()][..])
        );
        assert_eq!(value.get(0), Some("a"));
        assert_eq!(value.get(1), Some("b"));
        assert_eq!(value.get(2), Some("c"));
        assert_eq!(value.get(3), None);
    }

    #[test]
    fn test_len() {
        let string_val = ArrayValue::string("hello");
        assert_eq!(string_val.len(), 5);

        let array_val = ArrayValue::array(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(array_val.len(), 2);
    }

    #[test]
    fn test_display() {
        let string_val = ArrayValue::string("hello");
        assert_eq!(string_val.to_string(), "hello");

        let array_val = ArrayValue::array(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(array_val.to_string(), "(a b)");
    }
}
