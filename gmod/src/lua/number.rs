use super::push_to_lua::PushToLua;

pub trait LuaNumeric: PushToLua {}

impl LuaNumeric for i8 {}
impl LuaNumeric for i16 {}
impl LuaNumeric for i32 {}
impl LuaNumeric for i64 {}
impl LuaNumeric for i128 {}
impl LuaNumeric for isize {}
impl LuaNumeric for u8 {}
impl LuaNumeric for u16 {}
impl LuaNumeric for u32 {}
impl LuaNumeric for u64 {}
impl LuaNumeric for u128 {}
impl LuaNumeric for usize {}
impl LuaNumeric for f32 {}
impl LuaNumeric for f64 {}
