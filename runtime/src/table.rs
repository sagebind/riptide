use super::string::RipString;
use super::value::Value;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt,
    iter::FromIterator,
    rc::Rc,
};

/// Implementation of a "table". Tables are used like a map or object.
///
/// Only string keys are allowed.
#[derive(Clone)]
pub struct Table {
    /// Tables are stored by reference instead of by value. We use reference
    /// counting as a simple mechanism for that. Reference cycles are not
    /// accounted for and will leak.
    ///
    /// Unlike all other value types, tables are internally mutable, so we are
    /// using a cell here to implement that.
    inner: Rc<RefCell<BTreeMap<RipString, Value>>>,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    /// Allocate a new table.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    fn id(&self) -> usize {
        self.inner.as_ptr() as usize
    }

    /// Get the value indexed by a key.
    ///
    /// If the key does not exist, `Nil` is returned.
    pub fn get(&self, key: impl AsRef<[u8]>) -> Value {
        self.inner.borrow().get(key.as_ref()).cloned().unwrap_or(Value::Nil)
    }

    /// Set the value for a given key, returning the old value.
    ///
    /// If `Nil` is given as the value, the key is unset.
    pub fn set(&self, key: impl Into<RipString>, value: impl Into<Value>) -> Value {
        let key = key.into();
        let value = value.into();

        match value {
            Value::Nil => self.inner.borrow_mut().remove(key.as_bytes()).unwrap_or(Value::Nil),
            value => self.inner.borrow_mut().insert(key, value).unwrap_or(Value::Nil),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = RipString> {
        self.inner.borrow().keys().cloned().collect::<Vec<RipString>>().into_iter()
    }
}

impl<K: Into<RipString>, V: Into<Value>> FromIterator<(K, V)> for Table {
    fn from_iter<I: IntoIterator<Item=(K, V)>>(iter: I) -> Self {
        Self {
            inner: Rc::new(RefCell::new(iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect())),
        }
    }
}

impl PartialEq for Table {
    fn eq(&self, rhs: &Table) -> bool {
        // Table equality is based on identity rather than value.
        Rc::ptr_eq(&self.inner, &rhs.inner)
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.borrow().fmt(f)
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<table@{:x}>", self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        let table = Table::new();

        assert!(table.get("foo") == Value::Nil);
        assert!(table.set("foo", "hello") == Value::Nil);
        assert!(table.get("foo") == "hello");
        assert!(table.get("foo") == Value::from("hello"));
    }
}
