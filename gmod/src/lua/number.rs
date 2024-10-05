use super::{LuaNumber, State, LUA_NUMBER_MAX_SAFE_INTEGER};

pub trait LuaPushNumber {
    fn lua_push_number(self, l: State);
}

impl LuaPushNumber for i8 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}

impl LuaPushNumber for i16 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}

impl LuaPushNumber for i32 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}

impl LuaPushNumber for i64 {
    fn lua_push_number(self, l: State) {
        if self.abs() <= LUA_NUMBER_MAX_SAFE_INTEGER {
            l.lua_push_number(self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl LuaPushNumber for i128 {
    fn lua_push_number(self, l: State) {
        l.push_string(&self.to_string());
    }
}

impl LuaPushNumber for isize {
    fn lua_push_number(self, l: State) {
        if self.abs() <= LUA_NUMBER_MAX_SAFE_INTEGER as isize {
            l.lua_push_number(self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl LuaPushNumber for u8 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}

impl LuaPushNumber for u16 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}

impl LuaPushNumber for u32 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}

impl LuaPushNumber for u64 {
    fn lua_push_number(self, l: State) {
        if self <= LUA_NUMBER_MAX_SAFE_INTEGER as u64 {
            l.lua_push_number(self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl LuaPushNumber for u128 {
    fn lua_push_number(self, l: State) {
        l.push_string(&self.to_string());
    }
}

impl LuaPushNumber for usize {
    fn lua_push_number(self, l: State) {
        if self <= LUA_NUMBER_MAX_SAFE_INTEGER as usize {
            l.lua_push_number(self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl LuaPushNumber for f32 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}

impl LuaPushNumber for f64 {
    fn lua_push_number(self, l: State) {
        l.lua_push_number(self as LuaNumber);
    }
}
