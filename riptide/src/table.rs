use fnv::FnvHashMap;
use std::cell::RefCell;
use string::RString;
use value::Value;

/// Implementation of a "table". Tables are used like a map or object.
///
/// Only string keys are allowed.
#[derive(Clone, Debug)]
pub struct Table {
    /// Internally a hashmap is used, but the implementation could vary.
    ///
    /// Unlike all other value types, tables are mutable, so we are using a cell here to implement that.
    map: RefCell<FnvHashMap<RString, Value>>,
}

impl Table {
    /// Allocate a new table.
    pub fn new() -> Self {
        Table {
            map: RefCell::new(FnvHashMap::default()),
        }
    }

    /// Get the value indexed by a key.
    ///
    /// If the key does not exist, `None` is returned.
    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<Value> {
        self.map.borrow().get(key.as_ref()).cloned()
    }

    /// Set the value for a given key.
    ///
    /// If `Nil` is given as the value, the key is unset.
    pub fn set(&mut self, key: impl Into<RString>, value: impl Into<Value>) -> Option<Value> {
        let value = value.into();

        match value {
            Value::Nil => self.map.borrow_mut().remove(key.into().as_bytes()),
            value => self.map.borrow_mut().insert(key.into(), value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        let mut table = Table::new();

        assert!(table.get("foo") == None);
        assert!(table.set("foo", "hello").is_none());
        assert!(table.get("foo").unwrap() == "hello");
        assert!(table.get("foo").unwrap() == Value::from("hello"));
    }
}
