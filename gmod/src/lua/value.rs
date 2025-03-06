use super::push_to_lua::PushToLua;

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Boolean(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    F32(f32),
    F64(f64),
    String(String),
    BinaryString(Vec<u8>),
}

impl PushToLua for Value {
    fn push_to_lua(&self, l: &super::State) {
        match self {
            Value::Nil => l.push_nil(),
            Value::Boolean(b) => l.push_boolean(*b),
            Value::I8(i) => l.push_number(*i as f64),
            Value::I16(i) => l.push_number(*i as f64),
            Value::I32(i) => l.push_number(*i as f64),
            Value::I64(i) => l.push_number(*i as f64),
            Value::I128(i) => l.push_number(*i as f64),
            Value::Isize(i) => l.push_number(*i as f64),
            Value::U8(i) => l.push_number(*i as f64),
            Value::U16(i) => l.push_number(*i as f64),
            Value::U32(i) => l.push_number(*i as f64),
            Value::U64(i) => l.push_number(*i as f64),
            Value::U128(i) => l.push_number(*i as f64),
            Value::Usize(i) => l.push_number(*i as f64),
            Value::F32(i) => l.push_number(*i as f64),
            Value::F64(i) => l.push_number(*i),
            Value::String(s) => l.push_string(s),
            Value::BinaryString(s) => l.push_binary_string(s),
        }
    }
}
