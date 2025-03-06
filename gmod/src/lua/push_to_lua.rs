use super::{LuaNumber, State, LUA_NUMBER_MAX_SAFE_INTEGER};

pub trait PushToLua {
    fn push_to_lua(&self, l: &State);
}

impl PushToLua for i8 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for i16 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for i32 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for i64 {
    fn push_to_lua(&self, l: &State) {
        if self.abs() <= LUA_NUMBER_MAX_SAFE_INTEGER {
            l.raw_push_number(*self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl PushToLua for i128 {
    fn push_to_lua(&self, l: &State) {
        if self.abs() <= LUA_NUMBER_MAX_SAFE_INTEGER as i128 {
            l.raw_push_number(*self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl PushToLua for isize {
    fn push_to_lua(&self, l: &State) {
        if self.abs() <= LUA_NUMBER_MAX_SAFE_INTEGER as isize {
            l.raw_push_number(*self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl PushToLua for u8 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for u16 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for u32 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for u64 {
    fn push_to_lua(&self, l: &State) {
        if *self <= LUA_NUMBER_MAX_SAFE_INTEGER as u64 {
            l.raw_push_number(*self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl PushToLua for u128 {
    fn push_to_lua(&self, l: &State) {
        if *self <= LUA_NUMBER_MAX_SAFE_INTEGER as u128 {
            l.raw_push_number(*self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl PushToLua for usize {
    fn push_to_lua(&self, l: &State) {
        if *self <= LUA_NUMBER_MAX_SAFE_INTEGER as usize {
            l.raw_push_number(*self as LuaNumber);
        } else {
            l.push_string(&self.to_string());
        }
    }
}

impl PushToLua for f32 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for f64 {
    fn push_to_lua(&self, l: &State) {
        l.raw_push_number(*self as LuaNumber);
    }
}

impl PushToLua for bool {
    fn push_to_lua(&self, l: &State) {
        l.push_boolean(*self);
    }
}

impl PushToLua for &str {
    fn push_to_lua(&self, l: &State) {
        l.push_string(self);
    }
}

impl PushToLua for String {
    fn push_to_lua(&self, l: &State) {
        l.push_string(&self);
    }
}
