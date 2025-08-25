use std::{
    fmt,
    sync::{Arc, Weak},
};

use crate::next_tick::next_tick;

use super::{LUA_NOREF, LUA_REFNIL};

pub struct DynamicLuaReference(i32);

impl DynamicLuaReference {
    fn new(value: i32) -> Self {
        Self(value)
    }

    pub(crate) fn raw(&self) -> i32 {
        self.0
    }
}

impl Drop for DynamicLuaReference {
    fn drop(&mut self) {
        let raw_val = self.0;
        next_tick(move |l| {
            // dereference the reference from the registry table when we are done with the lua reference
            l.dereference(raw_val);
        });
    }
}

#[derive(Clone)]
pub enum LuaReference {
    Static(i32),
    Dynamic(Arc<DynamicLuaReference>),
}

impl LuaReference {
    pub fn new(reference: i32) -> Self {
        Self::Dynamic(Arc::new(DynamicLuaReference::new(reference)))
    }

    pub const fn new_static(value: i32) -> Self {
        Self::Static(value)
    }

    pub fn new_from_arc(reference: Arc<DynamicLuaReference>) -> Self {
        Self::Dynamic(reference)
    }

    #[inline(always)]
    pub fn raw(&self) -> i32 {
        match self {
            LuaReference::Static(value) => *value,
            LuaReference::Dynamic(value) => value.raw(),
        }
    }

    pub fn as_static(&self) -> Self {
        match self {
            LuaReference::Static(value) => Self::Static(*value),
            LuaReference::Dynamic(value) => Self::new_static(value.raw()),
        }
    }

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        let raw = self.raw();
        raw != LUA_NOREF && raw != LUA_REFNIL
    }

    #[inline(always)]
    pub fn weak_ref(&self) -> Weak<DynamicLuaReference> {
        match self {
            LuaReference::Static(_) => {
                panic!("Cannot get weak reference from static reference!")
            }
            LuaReference::Dynamic(value) => Arc::downgrade(value),
        }
    }
}

impl PartialEq for LuaReference {
    fn eq(&self, other: &Self) -> bool {
        self.raw() == other.raw()
    }
}

impl PartialEq<i32> for LuaReference {
    fn eq(&self, other: &i32) -> bool {
        self.raw() == *other
    }
}

impl PartialEq<LuaReference> for i32 {
    fn eq(&self, other: &LuaReference) -> bool {
        *self == other.raw()
    }
}

impl fmt::Display for LuaReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw())
    }
}

impl fmt::Debug for LuaReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LuaReference({})", self.raw())
    }
}

impl std::convert::AsRef<LuaReference> for LuaReference {
    fn as_ref(&self) -> &LuaReference {
        self
    }
}
