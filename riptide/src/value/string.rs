use std::borrow::Borrow;
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
pub enum RString<'s> {
    Borrowed(&'s [u8]),
    Heap(Rc<Vec<u8>>),
}

impl<'s> RString<'s> {
    /// The empty string.
    pub const EMPTY: Self = RString::Borrowed(b"");

    /// Get a view of the raw bytes in the string.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            &RString::Borrowed(s) => s,
            &RString::Heap(ref ptr) => ptr.as_ref(),
        }
    }

    pub fn as_utf8(&self) -> Option<&str> {
        str::from_utf8(self.as_bytes()).ok()
    }

    pub fn to_owned(&self) -> RString<'static> {
        self.as_bytes().to_owned().into()
    }

    pub fn to_lowercase(&self) -> RString<'static> {
        self.as_bytes().to_ascii_lowercase().into()
    }
}

impl<'s> From<&'s str> for RString<'s> {
    fn from(value: &'s str) -> Self {
        value.as_bytes().into()
    }
}

impl From<String> for RString<'static> {
    fn from(value: String) -> Self {
        value.into_bytes().into()
    }
}

impl<'s> From<&'s [u8]> for RString<'s> {
    fn from(value: &'s [u8]) -> Self {
        RString::Borrowed(value)
    }
}

impl From<Vec<u8>> for RString<'static> {
    fn from(value: Vec<u8>) -> Self {
        RString::Heap(Rc::new(value))
    }
}

impl<'s> AsRef<[u8]> for RString<'s> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'s> Borrow<[u8]> for RString<'s> {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'s> PartialEq for RString<'s> {
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

impl<'s> PartialEq<str> for RString<'s> {
    fn eq(&self, rhs: &str) -> bool {
        self == rhs.as_bytes()
    }
}

impl<'s> PartialEq<[u8]> for RString<'s> {
    fn eq(&self, rhs: &[u8]) -> bool {
        self.as_ref() == rhs
    }
}

impl<'s> Hash for RString<'s> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl<'s> fmt::Display for RString<'s> {
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
