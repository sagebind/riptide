use std::cell::RefCell;
use std::collections::HashMap;
use super::string::RString;
use super::Value;

/// Implementation of a "table". Tables are used like a map or object.
///
/// Only string keys are allowed.
#[derive(Clone, Debug)]
pub struct Table {
    /// Internally a hashmap is used, but the implementation could vary.
    ///
    /// Unlike all other value types, tables are mutable, so we are using a cell here to implement that.
    map: RefCell<HashMap<RString, Value>>,
}

impl Table {
    /// Allocate a new table.
    pub fn new() -> Self {
        Table {
            map: RefCell::new(HashMap::new()),
        }
    }

    /// Get the value indexed by a key.
    ///
    /// If the key does not exist, `Nil` is returned.
    pub fn get(&self, key: &str) -> Value {
        self.map.borrow().get(key).cloned().unwrap_or(Value::Nil)
    }

    /// Set the value for a given key.
    ///
    /// If `Nil` is given as the value, the key is unset.
    pub fn set<V: Into<Value>>(&mut self, key: &str, value: V) -> Option<Value> {
        let value = value.into();

        match value {
            Value::Nil => self.map.borrow_mut().remove(key),
            value => self.map.borrow_mut().insert(RString::from(String::from(key)), value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        let mut table = Table::new();

        assert!(table.get("foo") == Value::Nil);
        assert!(table.set("foo", "hello").is_none());
        assert!(table.get("foo") == "hello");
        assert!(table.get("foo") == Value::from("hello"));
    }
}
