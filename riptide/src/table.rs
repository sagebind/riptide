use crate::string::RipString;
use crate::value::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::iter::FromIterator;
use std::ptr;

/// Implementation of a "table". Tables are used like a map or object.
///
/// Only string keys are allowed.
#[derive(Clone)]
pub struct Table {
    /// Unlike all other value types, tables are internally mutable, so we are using a cell here to implement that.
    map: RefCell<BTreeMap<RipString, Value>>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            map: RefCell::new(BTreeMap::default()),
        }
    }
}

impl Table {
    /// Allocate a new table.
    pub fn new() -> Self {
        Self::default()
    }

    fn id(&self) -> usize {
        &self.map as *const _ as usize
    }

    /// Get the value indexed by a key.
    ///
    /// If the key does not exist, `Nil` is returned.
    pub fn get(&self, key: impl AsRef<[u8]>) -> Value {
        self.map.borrow().get(key.as_ref()).cloned().unwrap_or(Value::Nil)
    }

    /// Set the value for a given key, returning the old value.
    ///
    /// If `Nil` is given as the value, the key is unset.
    pub fn set(&self, key: impl Into<RipString>, value: impl Into<Value>) -> Value {
        let key = key.into();
        let value = value.into();

        match value {
            Value::Nil => self.map.borrow_mut().remove(key.as_bytes()).unwrap_or(Value::Nil),
            value => self.map.borrow_mut().insert(key, value).unwrap_or(Value::Nil),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = RipString> {
        self.map.borrow().keys().cloned().collect::<Vec<RipString>>().into_iter()
    }
}

impl<K: Into<RipString>, V: Into<Value>> FromIterator<(K, V)> for Table {
    fn from_iter<I: IntoIterator<Item=(K, V)>>(iter: I) -> Self {
        Self {
            map: RefCell::new(iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()),
        }
    }
}

impl PartialEq for Table {
    fn eq(&self, rhs: &Table) -> bool {
        // Table equality is based on identity rather than value.
        ptr::eq(self, rhs)
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.map.borrow().fmt(f)
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
