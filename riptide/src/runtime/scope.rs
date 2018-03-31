use value::Value;
use value::table::Table;

pub struct Scope {
    globals: Table,
    stack: Vec<CallFrame>,
}

pub struct CallFrame {
    args: Vec<Value>,
    bindings: Table,
}
