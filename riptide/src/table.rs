use crate::string::RipString;
use crate::value::Value;
use fnv::FnvHashMap;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// Implementation of a "table". Tables are used like a map or object.
///
/// Only string keys are allowed.
#[derive(Clone)]
pub struct Table {
    /// Internally a hashmap is used, but the implementation could vary.
    ///
    /// Unlike all other value types, tables are mutable, so we are using a cell here to implement that.
    map: Rc<RefCell<FnvHashMap<RipString, Value>>>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            map: Rc::new(RefCell::new(FnvHashMap::default())),
        }
    }
}

impl Table {
    /// Allocate a new table.
    pub fn new() -> Self {
        Self::default()
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
        let value = value.into();

        match value {
            Value::Nil => self.map.borrow_mut().remove(key.into().as_bytes()).unwrap_or(Value::Nil),
            value => self.map.borrow_mut().insert(key.into(), value).unwrap_or(Value::Nil),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = RipString> {
        self.map.borrow().keys().cloned().collect::<Vec<RipString>>().into_iter()
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.map.borrow().fmt(f)
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
