use std::borrow::*;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str;
use utf8;

/// A string value.
///
/// Strings are really just byte arrays and do not force any particular encoding, though UTF-8 is assumed when
/// displaying them.
///
/// Since strings are copied and tossed around quite a bit, the string is
/// reference counted to reduce memory and copying.
#[derive(Clone, Debug, Eq)]
pub enum RString {
    Static(&'static [u8]),
    Heap(Rc<Vec<u8>>),
}

impl RString {
    /// The empty string.
    pub const EMPTY: Self = RString::Static(b"");

    /// Allocate a new string and populate it with the given data.
    pub fn allocate(string: impl Into<Vec<u8>>) -> Self {
        RString::Heap(Rc::new(string.into()))
    }

    /// Create a string from a static Rust string.
    pub fn from_static(string: &'static str) -> Self {
        RString::Static(string.as_bytes())
    }

    /// Get a view of the raw bytes in the string.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            &RString::Static(s) => s,
            &RString::Heap(ref ptr) => ptr.as_ref(),
        }
    }

    pub fn as_utf8(&self) -> Option<&str> {
        str::from_utf8(self.as_bytes()).ok()
    }

    pub fn to_lowercase(&self) -> Self {
        self.as_bytes().to_ascii_lowercase().into()
    }
}

impl<'s> From<&'s str> for RString {
    fn from(value: &'s str) -> Self {
        Self::allocate(value)
    }
}

impl From<String> for RString {
    fn from(value: String) -> Self {
        Self::allocate(value)
    }
}

impl<'s> From<&'s [u8]> for RString {
    fn from(value: &'s [u8]) -> Self {
        Self::allocate(value)
    }
}

impl From<Vec<u8>> for RString {
    fn from(value: Vec<u8>) -> Self {
        Self::allocate(value)
    }
}

impl AsRef<[u8]> for RString {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<[u8]> for RString {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PartialEq for RString {
    fn eq(&self, rhs: &RString) -> bool {
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

impl PartialEq<str> for RString {
    fn eq(&self, rhs: &str) -> bool {
        self == rhs.as_bytes()
    }
}

impl PartialEq<[u8]> for RString {
    fn eq(&self, rhs: &[u8]) -> bool {
        self.as_bytes() == rhs
    }
}

impl Hash for RString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl fmt::Display for RString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut slice = self.as_bytes();

        while slice.len() > 0 {
            match utf8::decode(slice) {
                Ok(s) => return write!(f, "{}", s),
                Err(utf8::DecodeError::Incomplete {valid_prefix, ..}) => {
                    write!(f, "{}", valid_prefix)?;
                    slice = &slice[valid_prefix.len()..];
                },
                Err(utf8::DecodeError::Invalid {valid_prefix, remaining_input, ..}) => {
                    write!(f, "{}\u{FFFD}", valid_prefix)?;
                    slice = remaining_input;
                },
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
        let string = RString::from(expected.clone());

        assert_eq!(string.to_string(), expected);
    }

    #[test]
    fn test_to_string_non_utf8() {
        let expected = vec![55, 254, 72];
        let string = RString::from(expected.clone());

        assert_eq!(string.to_string(), "7�H");
    }
}
