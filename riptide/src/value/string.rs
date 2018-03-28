use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

/// A string value.
///
/// Since strings are copied and tossed around quite a bit, the string is
/// reference counted to reduce memory and copying.
#[derive(Clone, Debug, Eq)]
pub enum RString {
    Constant(&'static str),
    Heap(Rc<String>),
}

impl RString {
    /// The empty string.
    pub const EMPTY: Self = RString::Constant("");
}

impl From<&'static str> for RString {
    fn from(value: &'static str) -> Self {
        RString::Constant(value)
    }
}

impl From<String> for RString {
    fn from(value: String) -> Self {
        RString::Heap(Rc::new(value.into()))
    }
}

impl Deref for RString {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            &RString::Constant(s) => s,
            &RString::Heap(ref ptr) => ptr.as_ref(),
        }
    }
}

impl AsRef<str> for RString {
    fn as_ref(&self) -> &str {
        &*self
    }
}

impl Borrow<str> for RString {
    fn borrow(&self) -> &str {
        &*self
    }
}

impl PartialEq for RString {
    fn eq(&self, rhs: &RString) -> bool {
        let lhs = self.as_ref();
        let rhs = rhs.as_ref();

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
        self.as_ref() == rhs
    }
}

impl Hash for RString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl fmt::Display for RString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
