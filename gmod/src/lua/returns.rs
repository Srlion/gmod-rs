use std::borrow::Cow;

use super::State;

pub trait HandleLuaFunctionReturn {
    fn handle_result(self, l: State) -> i32;
}

impl HandleLuaFunctionReturn for i32 {
    #[inline(always)]
    fn handle_result(self, _: State) -> i32 {
        self
    }
}

impl<E: DisplayLuaError> HandleLuaFunctionReturn for Result<i32, E> {
    #[inline(always)]
    fn handle_result(self, l: State) -> i32 {
        match self {
            Ok(vals) => vals,
            // SAFETY: using the #[lua_function] macro, we are ONLY erroring AFTER the function returns
            // which means that longjmp won't mess up rust's stack
            Err(err) => l.error(err.display_lua_error().as_ref()),
        }
    }
}

impl<E: DisplayLuaError> HandleLuaFunctionReturn for Result<(), E> {
    #[inline(always)]
    fn handle_result(self, l: State) -> i32 {
        match self {
            Ok(_) => 0,
            // SAFETY: using the #[lua_function] macro, we are ONLY erroring AFTER the function returns
            // which means that longjmp won't mess up rust's stack
            Err(err) => l.error(err.display_lua_error().as_ref()),
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
