use std::{borrow::Cow, num::NonZeroI32};

use super::State;

pub trait HandleLuaFunctionReturn {
    fn handle_result(self, l: State) -> i32;
}

impl HandleLuaFunctionReturn for i32 {
    #[inline(always)]
    fn handle_result(self, l: State) -> i32 {
        self
    }
}

impl<E: DisplayLuaError> HandleLuaFunctionReturn for Result<i32, E> {
    #[inline(always)]
    fn handle_result(self, l: State) -> i32 {
        match self {
            Ok(vals) => vals,
            Err(err) => unsafe { l.error(err.display_lua_error().as_ref()) },
        }
    }
}

impl<E: DisplayLuaError> HandleLuaFunctionReturn for Result<(), E> {
    #[inline(always)]
    fn handle_result(self, l: State) -> i32 {
        match self {
            Ok(_) => 0,
            Err(err) => unsafe { l.error(err.display_lua_error().as_ref()) },
        }
    }
}

pub trait DisplayLuaError {
    fn display_lua_error(&self) -> Cow<'_, str>;
}
impl<E: std::fmt::Debug> DisplayLuaError for E {
    #[inline(always)]
    fn display_lua_error(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{:?}", self))
    }
}
