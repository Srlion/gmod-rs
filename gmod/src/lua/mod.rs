#![allow(static_mut_refs)]
#[allow(unused)]
use std::cell::UnsafeCell;
use std::sync::atomic::AtomicBool;

mod import;
pub use import::*;

mod lua_state;
pub use lua_state::LuaCStr;
pub use lua_state::LuaState as State;

mod returns;
pub use returns::HandleLuaFunctionReturn;

mod number;
pub mod push_to_lua;
mod value;
pub use value::Value;

pub mod rstruct;

pub mod reference;
pub use reference::LuaReference;

pub const LUA_NUMBER_MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

#[derive(Debug, Clone)]
pub enum LuaError {
    /// Out of memory
    ///
    /// `LUA_ERRMEM`
    MemoryAllocationError,

    /// A syntax error occurred in the passed Lua source code.
    ///
    /// `LUA_ERRSYNTAX`
    SyntaxError(Option<String>),

    /// Lua failed to load the given file.
    ///
    /// `LUA_ERRFILE`
    FileError(Option<String>),

    /// A runtime error occurred.
    ///
    /// `LUA_ERRRUN`
    RuntimeError(Option<String>),

    /// An error occurred while running the error handler function.
    ///
    /// `LUA_ERRERR`
    ErrorHandlerError,

    /// Tried to call a non-function value
    InvalidFunction,

    /// Unknown Lua error code
    Unknown(i32),
}

impl std::fmt::Display for LuaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LuaError::MemoryAllocationError => write!(f, "Out of memory"),
            LuaError::SyntaxError(Some(s)) => write!(f, "Syntax error: {}", s),
            LuaError::SyntaxError(None) => write!(f, "Syntax error"),
            LuaError::FileError(Some(s)) => write!(f, "File error: {}", s),
            LuaError::FileError(None) => write!(f, "File error"),
            LuaError::RuntimeError(Some(s)) => write!(f, "{}", s),
            LuaError::RuntimeError(None) => write!(f, "Runtime error"),
            LuaError::ErrorHandlerError => write!(f, "Error handler error"),
            LuaError::InvalidFunction => write!(f, "Tried to call a non-function value"),
            LuaError::Unknown(i) => write!(f, "Unknown Lua error code: {}", i),
        }
    }
}

impl std::error::Error for LuaError {}

/// Enforces a debug assertion that the Lua stack is unchanged after this block of code is executed.
///
/// Useful for ensuring stack hygiene.
///
/// `lua` is the Lua state to check.
///
/// # Example
///
/// ```rust,norun
/// lua_stack_guard!(lua => {
///     lua.get_global(lua_string!("hook"));
///     lua.get_field(-1, lua_string!("Add"));
///     lua.push_string("PlayerInitialSpawn");
///     lua.push_string("RustHook");
///     lua.push_function(player_initial_spawn);
///     lua.call(3, 0);
///     // lua.pop();
/// });
/// // PANIC: stack is dirty! We forgot to pop the hook library off the stack.
/// ```
#[macro_export]
macro_rules! lua_stack_guard {
    ( $lua:ident => $code:block ) => {{
        #[cfg(debug_assertions)]
        {
            let top = $lua.get_top();
            let ret = $code;
            if top != $lua.get_top() {
                $lua.dump_stack();
                panic!(
                    "Stack is dirty! Expected the stack to have {} elements, but it has {}!",
                    top,
                    $lua.get_top()
                );
            }
            ret
        }

        #[cfg(not(debug_assertions))]
        $code
    }};

    ( $lua:ident => $elem:literal => $code:block ) => {{
        #[cfg(debug_assertions)]
        {
            let ret = (|| $code)();
            if $lua.get_top() != $elem {
                $lua.dump_stack();
                panic!(
                    "Stack is dirty! Expected the stack to have ",
                    $elem,
                    " (fixed size) elements, but it has {}!",
                    $lua.get_top()
                );
            }
            ret
        }

        #[cfg(not(debug_assertions))]
        $code
    }};
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LuaDebug {
    pub event: i32,
    pub name: LuaString,
    pub namewhat: LuaString,
    pub what: LuaString,
    pub source: LuaString,
    pub currentline: i32,
    pub nups: i32,
    pub linedefined: i32,
    pub lastlinedefined: i32,
    pub short_src: [std::os::raw::c_char; LUA_IDSIZE],
    pub i_ci: i32,

    /* Extended fields. Only valid if lj_debug_getinfo() is called with ext = 1.*/
    pub nparams: i32,
    pub isvararg: i32,
}

static GMOD_CLOSED: AtomicBool = AtomicBool::new(false);

pub fn is_open() -> bool {
    !is_closed()
}

pub fn is_closed() -> bool {
    GMOD_CLOSED.load(std::sync::atomic::Ordering::Acquire)
}

pub fn set_closed() {
    GMOD_CLOSED.store(true, std::sync::atomic::Ordering::Release);
}

/// Loads lua_shared and imports all functions. This is already done for you if you add `#[gmod::gmod13_open]` to your `gmod13_open` function.
pub fn load(l: State) {
    unsafe { import::LUA_SHARED.load() };

    #[cfg(feature = "tokio-tasks")]
    super::tokio_tasks::load(l);

    rstruct::load(l);
    super::next_tick::load(l);
    GMOD_CLOSED.store(false, std::sync::atomic::Ordering::Release);
}

pub unsafe fn post_load(l: State) {
    rstruct::post_load(l);
}

pub fn unload(l: State) {
    super::next_tick::unload(l);

    #[cfg(feature = "tokio-tasks")]
    super::tokio_tasks::unload(l);

    // set_closed is called in the #[gmod13_close] macro, because unload is deferred
    rstruct::unload(l);
    unsafe { import::LUA_SHARED.unload() }
}
