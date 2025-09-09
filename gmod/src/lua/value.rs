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
            Value::Boolean(b) => b.push_to_lua(l),
            Value::I8(i) => i.push_to_lua(l),
            Value::I16(i) => i.push_to_lua(l),
            Value::I32(i) => i.push_to_lua(l),
            Value::I64(i) => i.push_to_lua(l),
            Value::I128(i) => i.push_to_lua(l),
            Value::Isize(i) => i.push_to_lua(l),
            Value::U8(i) => i.push_to_lua(l),
            Value::U16(i) => i.push_to_lua(l),
            Value::U32(i) => i.push_to_lua(l),
            Value::U64(i) => i.push_to_lua(l),
            Value::U128(i) => i.push_to_lua(l),
            Value::Usize(i) => i.push_to_lua(l),
            Value::F32(i) => i.push_to_lua(l),
            Value::F64(i) => i.push_to_lua(l),
            Value::String(s) => s.push_to_lua(l),
            Value::BinaryString(s) => l.push_binary_string(s),
        }
    }
}
