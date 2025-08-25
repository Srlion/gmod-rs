use std::{borrow::Cow, ffi::c_void, mem::MaybeUninit};

use anyhow::{bail, Result};
use number::LuaNumeric;

use crate::{lua::*, rstr};

use super::push_to_lua::PushToLua;

pub type LuaCStr<'a> = &'a std::ffi::CStr;

pub unsafe fn cast_cstr_to_static(s: LuaCStr<'_>) -> &'static std::ffi::CStr {
    std::mem::transmute(s)
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LuaState(pub *mut std::ffi::c_void);

impl LuaState {
    pub unsafe fn new() -> Result<Self, LuaError> {
        let lua = (LUA_SHARED.lual_newstate)();
        (LUA_SHARED.lual_openlibs)(lua);
        if lua.is_null() {
            Err(LuaError::MemoryAllocationError)
        } else {
            Ok(lua)
        }
    }

    pub fn register(&self, libname: LuaString, l: *const LuaReg) {
        unsafe { (LUA_SHARED.lual_register)(*self, libname, l) }
    }

    /// Returns whether this is the clientside Lua state or not.
    pub unsafe fn is_client(&self) -> bool {
        self.get_global(c"CLIENT");
        let client = self.get_boolean(-1);
        self.pop();
        client
    }

    /// Returns whether this is the serverside Lua state or not.
    pub unsafe fn is_server(&self) -> bool {
        self.get_global(c"SERVER");
        let server = self.get_boolean(-1);
        self.pop();
        server
    }

    /// Returns whether this is the menu Lua state or not.
    pub unsafe fn is_menu(&self) -> bool {
        self.get_global(c"MENU_DLL");
        let menu = self.get_boolean(-1);
        self.pop();
        menu
    }

    /// Returns the Lua string as a slice of bytes.
    ///
    /// Returns None if the value at the given index is not convertible to a string.
    pub fn get_binary_string(&self, index: i32) -> Option<Vec<u8>> {
        if !self.is_string(index) {
            return None;
        }
        let mut len: usize = 0;
        let ptr = unsafe { (LUA_SHARED.lua_tolstring)(*self, index, &mut len) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }.to_vec())
    }

    /// Returns the Lua string as a Rust UTF-8 String.
    ///
    /// Returns None if the value at the given index is not convertible to a string.
    ///
    /// This is a lossy operation, and will replace any invalid UTF-8 sequences with the Unicode replacement character. See the documentation for `String::from_utf8_lossy` for more information.
    ///
    /// If you need raw data, use `get_binary_string`.
    pub fn get_string(&self, index: i32) -> Option<String> {
        let str = self.get_binary_string(index)?;
        Some(String::from_utf8_lossy(&str).to_string())
    }

    pub fn get_string_unchecked(&self, index: i32) -> String {
        let str = self.get_binary_string(index).unwrap();
        String::from_utf8_lossy(&str).to_string()
    }

    /// Returns the name of the type of the value at the given index.
    pub unsafe fn get_type(&self, index: i32) -> &str {
        let lua_type = (LUA_SHARED.lua_type)(*self, index);
        let lua_type_str_ptr = (LUA_SHARED.lua_typename)(*self, lua_type);
        let lua_type_str = std::ffi::CStr::from_ptr(lua_type_str_ptr);
        unsafe { std::str::from_utf8_unchecked(lua_type_str.to_bytes()) }
    }

    #[inline(always)]
    pub fn get_top(&self) -> i32 {
        unsafe { (LUA_SHARED.lua_gettop)(*self) }
    }

    pub fn new_userdata(&self, size: usize) -> *mut c_void {
        unsafe { (LUA_SHARED.lua_newuserdata)(*self, size) }
    }

    #[inline(always)]
    pub fn push_struct<T: rstruct::RStruct>(&self, rstruct: T) {
        rstruct::push_struct(*self, rstruct);
    }

    #[inline(always)]
    pub fn get_struct<T: rstruct::RStruct>(&self, idx: i32) -> Result<&mut T> {
        rstruct::get_struct(*self, idx)
    }

    #[inline]
    pub fn raw_reference(&self) -> i32 {
        unsafe { (LUA_SHARED.lual_ref)(*self, LUA_REGISTRYINDEX) }
    }

    #[inline]
    pub fn raw_getref(&self, r#ref: i32) {
        self.raw_geti(LUA_REGISTRYINDEX, r#ref)
    }

    #[inline(always)]
    /// Pops the stack, inserts the value into the registry table, and returns the registry index of the value.
    ///
    /// Use `from_reference` with the reference index to push the value back onto the stack.
    ///
    /// Use `dereference` to free the reference from the registry table.
    pub fn reference(&self) -> LuaReference {
        let lua_ref = unsafe { (LUA_SHARED.lual_ref)(*self, LUA_REGISTRYINDEX) };
        LuaReference::new(lua_ref)
    }

    #[inline(always)]
    pub(crate) fn dereference(&self, r#ref: i32) {
        if r#ref == LUA_REFNIL || r#ref == LUA_NOREF {
            return;
        }
        unsafe { (LUA_SHARED.lual_unref)(*self, LUA_REGISTRYINDEX, r#ref) }
    }

    /// If it's a LUA_REFNIL or LUA_NOREF, it returns false and leaves the stack unchanged
    /// Otherwise, it pushes the reference to the stack and returns true
    #[inline(always)]
    pub fn from_reference<R: AsRef<LuaReference>>(&self, r#ref: R) -> bool {
        let r#ref = r#ref.as_ref().raw();
        if r#ref == LUA_REFNIL || r#ref == LUA_NOREF {
            return false;
        }
        self.raw_geti(LUA_REGISTRYINDEX, r#ref);
        true
    }

    #[inline(always)]
    /// You may be looking for `is_none_or_nil`
    pub fn is_nil(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_type)(*self, index) == LUA_TNIL }
    }

    #[inline(always)]
    pub fn is_none(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_type)(*self, index) == LUA_TNONE }
    }

    #[inline(always)]
    pub fn is_none_or_nil(&self, index: i32) -> bool {
        self.is_nil(index) || self.is_none(index)
    }

    #[inline(always)]
    pub fn is_function(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_type)(*self, index) == LUA_TFUNCTION }
    }

    #[inline(always)]
    pub fn is_table(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_type)(*self, index) == LUA_TTABLE }
    }

    #[inline(always)]
    pub fn is_boolean(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_type)(*self, index) == LUA_TBOOLEAN }
    }

    #[inline(always)]
    pub fn is_userdata(&self, index: i32) -> bool {
        unsafe {
            let ty = (LUA_SHARED.lua_type)(*self, index);
            ty == LUA_TUSERDATA || ty == LUA_TLIGHTUSERDATA
        }
    }

    #[inline(always)]
    pub fn is_string(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_type)(*self, index) == LUA_TSTRING }
    }

    #[inline(always)]
    pub fn is_number(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_type)(*self, index) == LUA_TNUMBER }
    }

    #[inline(always)]
    pub unsafe fn remove(&self, index: i32) {
        (LUA_SHARED.lua_remove)(*self, index)
    }

    #[inline(always)]
    pub fn push_value(&self, index: i32) {
        unsafe { (LUA_SHARED.lua_pushvalue)(*self, index) }
    }

    #[inline(always)]
    pub fn push_lightuserdata(&self, data: *mut c_void) {
        unsafe { (LUA_SHARED.lua_pushlightuserdata)(*self, data) }
    }

    #[inline(always)]
    pub fn push_to_lua<T: PushToLua>(&self, value: T) {
        value.push_to_lua(self);
    }

    #[inline(always)]
    pub fn raw_get_field(&self, index: i32, k: LuaCStr) {
        unsafe { (LUA_SHARED.lua_getfield)(*self, index, k.as_ptr()) };
    }

    pub fn get_field(&self, index: i32, key: LuaCStr) {
        static mut PROT_KEY: LuaCStr = c"NULL";
        unsafe extern "C-unwind" fn protected_helper(l: State) -> i32 {
            l.raw_get_field(1, PROT_KEY);
            1
        }

        if self.get_meta_field(index, c"__index") == 0 {
            self.raw_get_field(index, key);
            return;
        }

        unsafe {
            // SAFETY: We are only accessing static data here
            PROT_KEY = cast_cstr_to_static(key);
        }

        self.pop(); // pop the metamethod
        self.push_value(index); // push the table, to not pop it when calling the protected function
        self.push_function(protected_helper);
        self.insert(-2); // insert the protected function
        if !self.raw_pcall_ignore(1, 1) {
            // push nil if an error occurred, mimicking the behavior of lua_getfield
            self.push_nil();
        }
    }

    #[inline(always)]
    pub fn push_boolean(&self, boolean: bool) {
        unsafe { (LUA_SHARED.lua_pushboolean)(*self, if boolean { 1 } else { 0 }) }
    }

    #[inline(always)]
    pub fn push_bool(&self, boolean: bool) {
        self.push_boolean(boolean);
    }

    #[inline(always)]
    pub fn push_number<N>(&self, num: N)
    where
        N: LuaNumeric,
    {
        num.push_to_lua(self);
    }

    #[inline(always)]
    pub fn raw_push_number(&self, num: LuaNumber) {
        unsafe { (LUA_SHARED.lua_pushnumber)(*self, num) }
    }

    #[inline(always)]
    pub fn push_nil(&self) {
        unsafe { (LUA_SHARED.lua_pushnil)(*self) }
    }

    #[inline(always)]
    pub fn push_thread(&self) -> i32 {
        unsafe { (LUA_SHARED.lua_pushthread)(*self) }
    }

    #[inline(always)]
    pub fn to_thread(&self, index: i32) -> State {
        unsafe { (LUA_SHARED.lua_tothread)(*self, index) }
    }

    #[inline(always)]
    pub fn raw_pcall(&self, nargs: i32, nresults: i32, errfunc: i32) -> Result<(), LuaError> {
        let lua_error_code = unsafe { (LUA_SHARED.lua_pcall)(*self, nargs, nresults, errfunc) };
        if lua_error_code == 0 {
            Ok(())
        } else {
            Err(LuaError::from_lua_state(*self, lua_error_code))
        }
    }

    #[inline(always)]
    pub fn raw_pcall_ignore(&self, nargs: i32, nresults: i32) -> bool {
        if let Err(err) = self.raw_pcall(nargs, nresults, 0) {
            self.error_no_halt(&err.to_string(), None);
            return false;
        }
        true
    }

    ///
    /// # Example
    ///
    /// ```ignore
    /// #[lua_function]
    /// fn foo(l: gmod::lua::State) -> anyhow::Result<()> {
    ///     l.pcall(|| {
    ///         l.push_string("hello");
    ///         1
    ///     });
    ///     Ok(())
    /// }
    /// ```
    #[inline(always)]
    pub fn pcall<'a, F>(&self, callback: F) -> Result<(), LuaError>
    where
        F: FnOnce() -> i32 + 'a,
    {
        if !self.is_function(-1) {
            self.pop(); // pop whatever value
            return Err(LuaError::InvalidFunction);
        }
        let top = self.get_top();
        let nresults = callback();
        let n_args = self.get_top() - top;
        self.raw_pcall(n_args, nresults, 0)
    }

    /// Same as pcall, but ignores any runtime error and calls `ErrorNoHaltWithStack` instead with the error message.
    ///
    /// Returns whether the execution was successful.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[lua_function]
    /// fn foo(l: gmod::lua::State) -> anyhow::Result<()> {
    ///     l.pcall_ignore(|| {
    ///         l.push_string("hello");
    ///         1
    ///     });
    ///     Ok(())
    /// }
    /// ```
    #[inline(always)]
    pub fn pcall_ignore<'a, F>(&self, callback: F) -> bool
    where
        F: FnOnce() -> i32 + 'a,
    {
        if let Err(err) = self.pcall(callback) {
            self.error_no_halt(&err.to_string(), None);
            return false;
        }
        true
    }

    /// Check if reference is valid, if it's then check if it's a function and call it.
    ///
    /// You push the arguments inside the callback and return the number of results expected from the call.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[lua_function]
    /// fn foo(l: gmod::lua::State) -> anyhow::Result<()> {
    ///     let func_ref = l.check_function(1)?;
    ///     l.pcall_func_ref(func_ref, || {
    ///         l.push_string("hello");
    ///         1
    ///     });
    ///     Ok(())
    /// }
    /// ```
    pub fn pcall_func_ref<'a, R, F>(&self, func_ref: R, callback: F) -> Result<(), LuaError>
    where
        F: FnOnce() -> i32 + 'a,
        R: AsRef<LuaReference>,
    {
        if !self.from_reference(func_ref) {
            return Err(LuaError::InvalidFunction);
        }
        self.pcall(callback)
    }

    /// Same as pcall_func_ref, but ignores any runtime error and calls `ErrorNoHaltWithStack` instead with the error message.
    ///
    /// You push the arguments inside the callback and return the number of results expected from the call.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[lua_function]
    /// fn foo(l: gmod::lua::State) -> anyhow::Result<()> {
    ///     let func_ref = l.check_function(1)?;
    ///     l.pcall_func_ref_ignore(func_ref, || {
    ///         l.push_string("hello");
    ///     });
    ///     Ok(())
    /// }
    /// ```
    pub fn pcall_ignore_func_ref<'a, R, F>(&self, func_ref: R, callback: F) -> bool
    where
        F: FnOnce() -> i32 + 'a,
        R: AsRef<LuaReference>,
    {
        if !self.from_reference(func_ref) {
            return false;
        }
        self.pcall_ignore(callback)
    }

    #[inline(always)]
    pub fn cpcall(&self, func: LuaFunction, ud: *mut c_void) -> Result<(), LuaError> {
        let lua_error_code = unsafe { (LUA_SHARED.lua_cpcall)(*self, func, ud) };
        if lua_error_code == 0 {
            Ok(())
        } else {
            Err(LuaError::from_lua_state(*self, lua_error_code))
        }
    }

    #[inline(always)]
    pub fn cpcall_ignore(&self, func: LuaFunction, ud: *mut c_void) -> bool {
        if let Err(err) = self.cpcall(func, ud) {
            self.error_no_halt(&err.to_string(), None);
            return false;
        }

        true
    }

    pub fn load_string(&self, src: LuaCStr) -> Result<(), LuaError> {
        let lua_error_code = unsafe { (LUA_SHARED.lual_loadstring)(*self, src.as_ptr()) };
        if lua_error_code == 0 {
            Ok(())
        } else {
            Err(LuaError::from_lua_state(*self, lua_error_code))
        }
    }

    pub unsafe fn load_buffer(&self, buff: &[u8], name: LuaCStr) -> Result<(), LuaError> {
        let lua_error_code = (LUA_SHARED.lual_loadbuffer)(
            *self,
            buff.as_ptr() as LuaString,
            buff.len(),
            name.as_ptr(),
        );
        if lua_error_code == 0 {
            Ok(())
        } else {
            Err(LuaError::from_lua_state(*self, lua_error_code))
        }
    }

    pub fn lual_traceback(&self, state1: State, level: i32) {
        unsafe { (LUA_SHARED.lual_traceback)(*self, state1, std::ptr::null(), level) }
    }

    pub fn get_traceback(&self, state1: State, level: i32) -> String {
        self.lual_traceback(state1, level);
        let traceback = self.get_string(-1).unwrap_or(String::from("Unknown error")); // this shouldn't happen but just in case
        self.pop();
        traceback
    }

    pub unsafe fn load_file(&self, path: LuaCStr) -> Result<(), LuaError> {
        let lua_error_code = (LUA_SHARED.lual_loadfile)(*self, path.as_ptr());
        if lua_error_code == 0 {
            Ok(())
        } else {
            Err(LuaError::from_lua_state(*self, lua_error_code))
        }
    }

    #[inline(always)]
    pub fn pop(&self) {
        self.pop_n(1);
    }

    #[inline(always)]
    pub fn pop_n(&self, count: i32) {
        self.set_top(-count - 1);
    }

    #[inline(always)]
    pub fn set_top(&self, index: i32) {
        unsafe { (LUA_SHARED.lua_settop)(*self, index) }
    }

    #[inline(always)]
    pub fn lua_type(&self, index: i32) -> i32 {
        unsafe { (LUA_SHARED.lua_type)(*self, index) }
    }

    pub fn lua_type_name<'a>(&self, lua_type_id: i32) -> Cow<'a, str> {
        unsafe {
            let type_str_ptr = (LUA_SHARED.lua_typename)(*self, lua_type_id);
            let type_str = std::ffi::CStr::from_ptr(type_str_ptr);

            type_str.to_string_lossy()
        }
    }

    #[inline(always)]
    pub fn replace(&self, index: i32) {
        unsafe { (LUA_SHARED.lua_replace)(*self, index) }
    }

    #[inline(always)]
    pub unsafe fn push_globals(&self) {
        (LUA_SHARED.lua_pushvalue)(*self, LUA_GLOBALSINDEX)
    }

    #[inline(always)]
    pub unsafe fn push_registry(&self) {
        (LUA_SHARED.lua_pushvalue)(*self, LUA_REGISTRYINDEX)
    }

    #[inline(always)]
    pub fn push_string(&self, data: &str) {
        unsafe { (LUA_SHARED.lua_pushlstring)(*self, data.as_ptr() as LuaString, data.len()) }
    }

    #[inline(always)]
    pub fn push_binary_string(&self, data: &[u8]) {
        unsafe { (LUA_SHARED.lua_pushlstring)(*self, data.as_ptr() as LuaString, data.len()) }
    }

    #[inline(always)]
    pub fn push_function(&self, func: LuaFunction) {
        unsafe { (LUA_SHARED.lua_pushcclosure)(*self, func, 0) }
    }

    #[inline(always)]
    /// Creates a closure, which can be used as a function with stored data (upvalues)
    ///
    /// ## Example
    ///
    /// ```ignore
    /// #[lua_function]
    /// unsafe fn foo(lua: gmod::lua::State) {
    ///     lua.get_closure_arg(1);
    ///     let hello = lua.get_string(-1);
    ///     println!("{}", hello);
    /// }
    ///
    /// lua.push_string("Hello, world!");
    /// lua.push_closure(foo, 1);
    /// ```
    pub fn push_closure(&self, func: LuaFunction, n: i32) {
        debug_assert!(
            n <= 255,
            "Can't push more than 255 arguments into a closure"
        );
        unsafe { (LUA_SHARED.lua_pushcclosure)(*self, func, n) }
    }

    #[inline(always)]
    /// Pushes the `n`th closure argument onto the stack
    ///
    /// ## Example
    ///
    /// ```ignore
    /// #[lua_function]
    /// unsafe fn foo(lua: gmod::lua::State) {
    ///     lua.push_closure_arg(1);
    ///     let hello = lua.get_string(-1);
    ///     println!("{}", hello);
    /// }
    ///
    /// lua.push_string("Hello, world!");
    /// lua.push_closure(foo, 1);
    /// ```
    pub fn push_closure_arg(&self, n: i32) {
        self.push_value(self.upvalue_index(n));
    }

    #[inline(always)]
    /// Equivalent to C `lua_upvalueindex` macro
    pub const fn upvalue_index(&self, idx: i32) -> i32 {
        LUA_GLOBALSINDEX - idx
    }

    /// TODO: apply the trick from set_field to make this protected from crashing
    #[inline(always)]
    pub fn raw_set_table(&self, index: i32) {
        unsafe { (LUA_SHARED.lua_settable)(*self, index) }
    }

    #[inline(always)]
    pub fn raw_set_field(&self, index: i32, k: LuaCStr) {
        unsafe { (LUA_SHARED.lua_setfield)(*self, index, k.as_ptr()) }
    }

    #[inline(always)]
    pub fn set_field(&self, index: i32, key: LuaCStr) {
        static mut PROT_KEY: LuaCStr = c"NULL";
        unsafe extern "C-unwind" fn protected_helper(l: State) -> i32 {
            l.raw_set_field(1, PROT_KEY); // 1 will be the table
            0
        }

        if self.get_meta_field(index, c"__newindex") == 0 {
            self.raw_set_field(index, key);
            return;
        }

        unsafe {
            // SAFETY: We are only accessing static data here
            PROT_KEY = cast_cstr_to_static(key);
        }

        self.pop(); // pop the metamethod
        self.push_value(index); // push the table, to not pop it when calling the protected function
        self.insert(-2); // move the table before the value that is on top of the stack
        self.push_function(protected_helper);
        self.insert(-3); // insert the protected function before the table
        self.raw_pcall_ignore(2, 0);
    }

    #[inline(always)]
    pub fn get_global(&self, name: LuaCStr) {
        self.get_field(LUA_GLOBALSINDEX, name)
    }

    #[inline(always)]
    pub fn set_global(&self, name: LuaCStr) {
        self.set_field(LUA_GLOBALSINDEX, name)
    }

    #[inline(always)]
    /// WARNING: Any Lua errors caused by calling the function will longjmp and prevent any further execution of your code.
    ///
    /// To workaround this, use `pcall_ignore`, which will call `ErrorNoHaltWithStack` instead and allow your code to continue executing.
    pub unsafe fn call(&self, nargs: i32, nresults: i32) {
        (LUA_SHARED.lua_call)(*self, nargs, nresults)
    }

    #[inline(always)]
    pub fn insert(&self, index: i32) {
        unsafe { (LUA_SHARED.lua_insert)(*self, index) }
    }

    /// Creates a new table and pushes it to the stack.
    /// seq_n is a hint as to how many sequential elements the table may have.
    /// hash_n is a hint as to how many non-sequential/hashed elements the table may have.
    /// Lua may use these hints to preallocate memory.
    #[inline(always)]
    pub fn create_table(&self, seq_n: i32, hash_n: i32) {
        unsafe { (LUA_SHARED.lua_createtable)(*self, seq_n, hash_n) }
    }

    /// Creates a new table and pushes it to the stack without memory preallocation hints.
    /// Equivalent to `create_table(0, 0)`
    #[inline(always)]
    pub fn new_table(&self) {
        unsafe { (LUA_SHARED.lua_createtable)(*self, 0, 0) }
    }

    /// TODO: apply the trick from get_field to make this protected from crashing
    #[inline(always)]
    pub fn raw_get_table(&self, index: i32) {
        unsafe { (LUA_SHARED.lua_gettable)(*self, index) }
    }

    pub unsafe fn check_binary_string(&self, arg: i32) -> Result<Vec<u8>> {
        match self.get_binary_string(arg) {
            Some(s) => Ok(s),
            None => bail!(self.tag_error(arg, LUA_TSTRING)),
        }
    }

    pub fn check_string(&self, arg: i32) -> Result<String> {
        match self.get_string(arg) {
            Some(s) => Ok(s),
            None => bail!(self.tag_error(arg, LUA_TSTRING)),
        }
    }

    // #[inline(always)]
    // pub unsafe fn check_userdata(&self, arg: i32, name: LuaCStr) -> Result<*mut c_void> {
    //     if self.test_userdata(arg, name) {
    //         Ok(self.to_userdata(arg))
    //     } else {
    //         bail!(self.tag_error(arg, LUA_TUSERDATA))
    //     }
    // }

    // pub unsafe fn test_userdata(&self, index: i32, name: LuaCStr) -> bool {
    //     if !(LUA_SHARED.lua_touserdata)(*self, index).is_null() && self.get_metatable(index) != 0 {
    //         self.get_field(LUA_REGISTRYINDEX, name);
    //         let result = self.raw_equal(-1, -2);
    //         self.pop_n(2);
    //         if result {
    //             return true;
    //         }
    //     }
    //     false
    // }

    #[inline(always)]
    pub fn raw_equal(&self, a: i32, b: i32) -> bool {
        unsafe { (LUA_SHARED.lua_rawequal)(*self, a, b) == 1 }
    }

    #[inline(always)]
    pub fn get_metatable_name(&self, name: LuaCStr) {
        unsafe { (LUA_SHARED.lua_getfield)(*self, LUA_REGISTRYINDEX, name.as_ptr()) }
    }

    #[inline(always)]
    pub fn get_metatable(&self, idx: i32) -> i32 {
        unsafe { (LUA_SHARED.lua_getmetatable)(*self, idx) }
    }

    #[inline(always)]
    pub fn check_table(&self, arg: i32) -> Result<()> {
        if self.is_table(arg) {
            Ok(())
        } else {
            bail!(self.tag_error(arg, LUA_TTABLE))
        }
    }

    #[inline(always)]
    pub fn check_function(&self, arg: i32) -> Result<LuaReference> {
        if self.is_function(arg) {
            self.push_value(arg);
            Ok(self.reference())
        } else {
            bail!(self.tag_error(arg, LUA_TFUNCTION))
        }
    }

    #[inline(always)]
    pub fn check_number(&self, arg: i32) -> Result<f64> {
        if self.is_number(arg) {
            Ok(self.to_number(arg))
        } else {
            bail!(self.tag_error(arg, LUA_TNUMBER))
        }
    }

    #[inline(always)]
    pub fn check_boolean(&self, arg: i32) -> Result<bool> {
        if self.is_boolean(arg) {
            Ok(self.get_boolean(arg))
        } else {
            bail!(self.tag_error(arg, LUA_TBOOLEAN))
        }
    }

    #[inline(always)]
    pub fn to_number(&self, index: i32) -> f64 {
        unsafe { (LUA_SHARED.lua_tonumber)(*self, index) }
    }

    #[inline(always)]
    pub fn get_boolean(&self, index: i32) -> bool {
        unsafe { (LUA_SHARED.lua_toboolean)(*self, index) == 1 }
    }

    #[inline(always)]
    pub fn set_metatable(&self, index: i32) -> i32 {
        unsafe { (LUA_SHARED.lua_setmetatable)(*self, index) }
    }

    #[inline(always)]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self, index: i32) -> i32 {
        unsafe { (LUA_SHARED.lua_objlen)(*self, index) }
    }

    #[inline(always)]
    pub fn raw_geti(&self, t: i32, index: i32) {
        unsafe { (LUA_SHARED.lua_rawgeti)(*self, t, index) };
    }

    #[inline(always)]
    pub fn raw_seti(&self, t: i32, index: i32) {
        unsafe { (LUA_SHARED.lua_rawseti)(*self, t, index) }
    }

    #[inline(always)]
    pub unsafe fn next(&self, index: i32) -> i32 {
        (LUA_SHARED.lua_next)(*self, index)
    }

    #[inline(always)]
    pub unsafe fn to_pointer(&self, index: i32) -> *const c_void {
        (LUA_SHARED.lua_topointer)(*self, index)
    }

    #[inline(always)]
    pub fn to_userdata(&self, index: i32) -> *mut c_void {
        unsafe { (LUA_SHARED.lua_touserdata)(*self, index) }
    }

    #[inline(always)]
    pub fn coroutine_new(&self) -> LuaState {
        unsafe { (LUA_SHARED.lua_newthread)(*self) }
    }

    #[inline(always)]
    /// Exchange values between different threads of the same global state.
    ///
    /// This function pops `n` values from the stack `self`, and pushes them onto the stack `target_thread`.
    pub fn coroutine_exchange(&self, target_thread: LuaState, n: i32) {
        unsafe { (LUA_SHARED.lua_xmove)(*self, target_thread, n) }
    }

    #[inline(always)]
    #[must_use]
    pub fn coroutine_yield(&self, nresults: i32) -> i32 {
        unsafe { (LUA_SHARED.lua_yield)(*self, nresults) }
    }

    #[inline(always)]
    #[must_use]
    pub fn coroutine_resume(&self, narg: i32) -> i32 {
        unsafe { (LUA_SHARED.lua_resume)(*self, narg) }
    }

    #[inline(always)]
    /// See `call`
    pub fn coroutine_resume_call(&self, narg: i32) {
        match self.coroutine_resume(narg) {
            LUA_OK => {}
            LUA_ERRRUN => self.error(self.get_string(-2).unwrap_or(String::from("Unknown error"))),
            LUA_ERRMEM => self.error("Out of memory"),
            _ => self.error("Unknown internal Lua error"),
        }
    }

    #[inline(always)]
    /// See `pcall_ignore`
    pub fn coroutine_resume_ignore(&self, narg: i32, traceback: Option<&str>) -> Result<i32, ()> {
        match self.coroutine_resume(narg) {
            status @ (LUA_OK | LUA_YIELD) => Ok(status),
            err => {
                let err = LuaError::from_lua_state(*self, err);
                self.error_no_halt(&err.to_string(), traceback);
                Err(())
            }
        }
    }

    #[inline(always)]
    pub fn coroutine_status(&self) -> i32 {
        unsafe { (LUA_SHARED.lua_status)(*self) }
    }

    #[inline(always)]
    pub fn equal(&self, index1: i32, index2: i32) -> bool {
        unsafe { (LUA_SHARED.lua_equal)(*self, index1, index2) == 1 }
    }

    #[inline(always)]
    pub fn setfenv(&self, index: i32) -> i32 {
        unsafe { (LUA_SHARED.lua_setfenv)(*self, index) }
    }

    #[inline(always)]
    pub fn getfenv(&self, index: i32) {
        unsafe { (LUA_SHARED.lua_getfenv)(*self, index) }
    }

    #[inline]
    pub fn get_meta_field(&self, index: i32, k: LuaCStr) -> i32 {
        unsafe { (LUA_SHARED.lual_getmetafield)(*self, index, k.as_ptr()) }
    }

    #[inline]
    pub fn get_calling_file_name(&self) -> Option<String> {
        if let Some(ar) = self.debug_getinfo_at(1, c"S") {
            unsafe {
                // SAFETY: short_src is guaranteed to be NUL-terminated by LuaJIT.
                return Some(
                    std::ffi::CStr::from_ptr(ar.source)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        None
    }

    /// Creates a new table in the registry with the given `name` as the key if it doesn't already exist, and pushes it onto the stack.
    ///
    /// Returns if the metatable was already present in the registry.
    #[inline(always)]
    pub fn new_metatable(&self, name: LuaCStr) -> bool {
        unsafe { (LUA_SHARED.lual_newmetatable)(*self, name.as_ptr()) == 0 }
    }

    // lua functions shouldn't be able to call it directly and should instead return Result types, as destructors may not be called
    #[cold]
    pub(crate) fn error<S: AsRef<str>>(&self, msg: S) -> ! {
        self.push_string(msg.as_ref());
        unsafe { (LUA_SHARED.lua_error)(*self) };
        unreachable!()
    }

    pub fn debug_getinfo_from_ar(&self, ar: &mut LuaDebug, what: LuaCStr) -> Result<(), ()> {
        unsafe {
            if (LUA_SHARED.lua_getinfo)(*self, what.as_ptr(), ar as *mut LuaDebug) != 0 {
                Ok(())
            } else {
                Err(())
            }
        }
    }

    /// `what` should start with `>` and pop a function off the stack
    pub unsafe fn debug_getinfo_from_stack(&self, what: LuaCStr) -> Option<LuaDebug> {
        let mut ar = MaybeUninit::uninit();
        if (LUA_SHARED.lua_getinfo)(*self, what.as_ptr(), ar.as_mut_ptr()) != 0 {
            Some(ar.assume_init())
        } else {
            None
        }
    }

    pub fn get_stack_at(&self, level: i32) -> Option<LuaDebug> {
        unsafe {
            let mut ar = MaybeUninit::uninit();
            if (LUA_SHARED.lua_getstack)(*self, level, ar.as_mut_ptr()) != 0 {
                Some(ar.assume_init())
            } else {
                None
            }
        }
    }

    pub fn debug_getinfo_at(&self, level: i32, what: LuaCStr) -> Option<LuaDebug> {
        unsafe {
            let mut ar = MaybeUninit::uninit();
            if (LUA_SHARED.lua_getstack)(*self, level, ar.as_mut_ptr()) != 0
                && (LUA_SHARED.lua_getinfo)(*self, what.as_ptr(), ar.as_mut_ptr()) != 0
            {
                return Some(ar.assume_init());
            }
            None
        }
    }

    pub fn dump_stack(&self) {
        let top = self.get_top();
        println!("\n=== STACK DUMP ===");
        println!("Stack size: {}", top);
        for i in 1..=top {
            let lua_type = self.lua_type(i);
            let lua_type_name = self.lua_type_name(lua_type);
            match lua_type_name.as_ref() {
                "string" => println!("{}. {}: {:?}", i, lua_type_name, {
                    self.push_value(i);
                    let str = self.get_string(-1);
                    self.pop();
                    str
                }),
                "boolean" => println!("{}. {}: {:?}", i, lua_type_name, {
                    self.push_value(i);
                    let bool = self.get_boolean(-1);
                    self.pop();
                    bool
                }),
                "number" => println!("{}. {}: {:?}", i, lua_type_name, {
                    self.push_value(i);
                    let n = self.to_number(-1);
                    self.pop();
                    n
                }),
                _ => println!("{}. {}", i, lua_type_name),
            }
        }
        println!();
    }

    pub unsafe fn dump_val(&self, index: i32) -> String {
        let lua_type_name = self.lua_type_name(self.lua_type(index));
        match lua_type_name.as_ref() {
            "string" => {
                self.push_value(index);
                let str = self.get_string(-1);
                self.pop();
                format!("{:?}", str.unwrap())
            }
            "boolean" => {
                self.push_value(index);
                let boolean = self.get_boolean(-1);
                self.pop();
                format!("{}", boolean)
            }
            "number" => {
                self.push_value(index);
                let n = self.to_number(-1);
                self.pop();
                format!("{}", n)
            }
            _ => lua_type_name.into_owned(),
        }
    }

    pub fn get_field_type_or_nil(&self, idx: i32, name: LuaCStr, ty: i32) -> Result<bool> {
        self.get_field(idx, name);

        if self.is_none_or_nil(-1) {
            self.pop();
            return Ok(false);
        }

        if self.lua_type(-1) != ty {
            self.pop();
            bail!(
                "bad type for field: '{}' ({} expected, got: {})",
                rstr!(name.as_ptr()),
                self.lua_type_name(ty),
                self.lua_type_name(self.lua_type(-1))
            );
        }

        Ok(true)
    }

    pub fn type_error(&self, narg: i32, tname: &str) -> String {
        let err = format!(
            "{} expected, got {}",
            tname,
            self.lua_type_name(self.lua_type(narg))
        );
        self.err_argmsg(narg, &err)
    }

    pub fn tag_error(&self, narg: i32, tag: i32) -> String {
        self.type_error(narg, &self.lua_type_name(tag))
    }

    pub fn err_argmsg(&self, mut narg: i32, msg: &str) -> String {
        let mut fname = "?";
        let mut namewhat: Option<&str> = None;

        if let Some(ar) = self.debug_getinfo_at(0, c"n") {
            if !ar.name.is_null() {
                fname = rstr!(ar.name);
            }
            if !ar.namewhat.is_null() {
                namewhat = Some(rstr!(ar.namewhat));
            }
        }

        if narg < 0 && narg > LUA_REGISTRYINDEX {
            narg = self.get_top() + narg + 1;
        }

        if let Some(namewhat) = namewhat {
            if namewhat == "method" && {
                narg -= 1;
                narg == 0
            } {
                return format!("bad self parameter in method '{}' ({})", fname, msg);
            }
        }

        format!("bad argument #{} to '{}' ({})", narg, fname, msg)
    }

    pub fn error_no_halt(&self, err: &str, traceback: Option<&str>) {
        let mut error_prefix = "[ERROR] ";
        let err = if let Some(traceback) = traceback {
            error_prefix = "";

            self.get_global(c"ErrorNoHalt");
            format!("[ERROR] {}\n{}\n", err, traceback)
        } else {
            self.get_global(c"ErrorNoHaltWithStack");
            err.to_string()
        };

        if self.is_nil(-1) {
            self.pop();
            eprintln!("{error_prefix}{err}");
        } else {
            self.push_string(&err);
            if self.raw_pcall(1, 0, 0).is_err() {
                eprintln!("{error_prefix}{err}");
            }
        }
    }
}
impl std::ops::Deref for LuaState {
    type Target = *mut std::ffi::c_void;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
