use bytes::Bytes;
use std::borrow::*;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str;

/// A string value.
///
/// Strings are really just byte arrays and do not force any particular encoding, though UTF-8 is assumed when
/// displaying them.
///
/// Since strings are copied and tossed around quite a bit, the string is reference counted to reduce memory and
/// copying.
#[derive(Clone, Eq)]
pub struct RipString(Bytes);

impl Default for RipString {
    fn default() -> Self {
        Self::from_static("")
    }
}

impl RipString {
    /// Allocate a new string and populate it with the given data.
    pub fn new(string: impl Into<Bytes>) -> Self {
        RipString(string.into())
    }

    /// Create a string from a static Rust string.
    pub fn from_static(string: &'static str) -> Self {
        RipString(Bytes::from_static(string.as_bytes()))
    }

    /// Get a view of the raw bytes in the string.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn as_utf8(&self) -> Option<&str> {
        str::from_utf8(self.as_bytes()).ok()
    }

    pub fn to_lowercase(&self) -> Self {
        self.as_bytes().to_ascii_lowercase().into()
    }
}

impl<'s> From<&'s str> for RipString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RipString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl<'s> From<&'s [u8]> for RipString {
    fn from(value: &'s [u8]) -> Self {
        Self::new(value)
    }
}

impl From<Vec<u8>> for RipString {
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}

impl From<Bytes> for RipString {
    fn from(value: Bytes) -> Self {
        Self::new(value)
    }
}

impl From<RipString> for Bytes {
    fn from(string: RipString) -> Self {
        string.0
    }
}

impl AsRef<[u8]> for RipString {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<[u8]> for RipString {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PartialEq for RipString {
    fn eq(&self, rhs: &RipString) -> bool {
        let lhs = self.as_bytes();
        let rhs = rhs.as_bytes();

        // First compare by address.
        if lhs as *const _ == rhs as *const _ {
            return true;
        }

        // Compare by string contents.
        lhs == rhs
    }
}

impl PartialEq<str> for RipString {
    fn eq(&self, rhs: &str) -> bool {
        self == rhs.as_bytes()
    }
}

impl PartialEq<[u8]> for RipString {
    fn eq(&self, rhs: &[u8]) -> bool {
        self.as_bytes() == rhs
    }
}

impl PartialOrd for RipString {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        self.as_bytes().partial_cmp(rhs.as_bytes())
    }
}

impl Ord for RipString {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.as_bytes().cmp(rhs.as_bytes())
    }
}

impl Hash for RipString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl fmt::Debug for RipString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for RipString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut slice = self.as_bytes();

        while slice.len() > 0 {
            match utf8::decode(slice) {
                Ok(s) => return write!(f, "{}", s),
                Err(utf8::DecodeError::Incomplete {
                    valid_prefix,
                    ..
                }) => {
                    write!(f, "{}", valid_prefix)?;
                    slice = &slice[valid_prefix.len()..];
                }
                Err(utf8::DecodeError::Invalid {
                    valid_prefix,
                    remaining_input,
                    ..
                }) => {
                    write!(f, "{}\u{FFFD}", valid_prefix)?;
                    slice = remaining_input;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string_utf8() {
        let expected = String::from("hello world");
        let string = RipString::from(expected.clone());

        assert_eq!(string.to_string(), expected);
    }

    #[test]
    fn test_to_string_non_utf8() {
        let expected = vec![55, 254, 72];
        let string = RipString::from(expected.clone());

        assert_eq!(string.to_string(), "7�H");
    }
}
