use bstr::BString;
use std::borrow::*;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str;

/// A string value.
///
/// Strings are really just byte arrays and do not force any particular encoding, though UTF-8 is assumed when
/// displaying them.
///
/// Since strings are copied and tossed around quite a bit, the string is reference counted to reduce memory and
/// copying.
#[derive(Clone, Eq)]
pub struct RipString(Rc<BString>);

impl Default for RipString {
    fn default() -> Self {
        Self::from("")
    }
}

impl RipString {
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
        RipString(Rc::new(value.into()))
    }
}

impl From<String> for RipString {
    fn from(value: String) -> Self {
        RipString(Rc::new(value.into()))
    }
}

impl<'s> From<&'s [u8]> for RipString {
    fn from(value: &'s [u8]) -> Self {
        RipString(Rc::new(value.into()))
    }
}

impl From<Vec<u8>> for RipString {
    fn from(value: Vec<u8>) -> Self {
        RipString(Rc::new(value.into()))
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

impl PartialEq<[u8]> for RipString {
    fn eq(&self, rhs: &[u8]) -> bool {
        self.as_bytes() == rhs
    }
}

impl PartialEq<str> for RipString {
    fn eq(&self, rhs: &str) -> bool {
        self == rhs.as_bytes()
    }
}

impl<'a> PartialEq<&'a str> for RipString {
    fn eq(&self, rhs: &&str) -> bool {
        self == rhs.as_bytes()
    }
}

impl PartialEq<String> for RipString {
    fn eq(&self, rhs: &String) -> bool {
        self == rhs.as_bytes()
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
        fmt::Display::fmt(&*self.0, f)
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
