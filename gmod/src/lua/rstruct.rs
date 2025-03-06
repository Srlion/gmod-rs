use std::{collections::HashMap, ffi::c_void, mem::MaybeUninit, sync::Weak};

use anyhow::{bail, Result};

use super::{reference::DynamicLuaReference, LuaCStr, LuaFunction, LuaReference, State};

pub type Methods = &'static [(LuaCStr<'static>, LuaFunction)];

pub struct StructMetaTable {
    pub name: LuaCStr<'static>,
    pub methods: Methods,
    pub post_gmod_open: fn(State),
}

// This is used as the key to store the userdata inside the lua table
const INDEX_KEY: i32 = 0x4B1D92A3;

#[linkme::distributed_slice]
pub static META_TABLES: [StructMetaTable];

pub trait RStruct: Sized + 'static {
    fn metatable_name(&self) -> LuaCStr<'static>;

    #[inline(always)]
    fn consume(l: State) -> Option<Box<Self>> {
        let ud_ptr = l.to_userdata(1);
        if let Some(obj) = get_map().remove(&ud_ptr) {
            let val = unsafe { Box::from_raw(obj.ptr as *mut Self) };
            return Some(val);
        }
        None
    }
    #[allow(non_snake_case)]
    extern "C-unwind" fn internal__gc(l: State) -> i32 {
        if let Some(val) = Self::consume(l) {
            drop(val); // explicit drop
        }
        0
    }

    fn post_gmod_open(_: State) {}
}

struct RStructInner {
    ptr: usize,                         // pointer to the Arc<T>
    ref_idx: Weak<DynamicLuaReference>, // reference to the userdata, if any
}

static mut LUA_STRUCTS: MaybeUninit<HashMap<*mut c_void, RStructInner>> = MaybeUninit::uninit();

pub(crate) fn load(l: State) {
    unsafe {
        LUA_STRUCTS.write(HashMap::new());
    }

    for metatable in META_TABLES {
        l.new_metatable(metatable.name);
        {
            for &(name, func) in metatable.methods {
                l.push_function(func);
                l.set_field(-2, name);
            }

            l.push_value(-1); // Pushes the metatable to the top of the stack
            l.set_field(-2, c"__index");
        }
        l.pop();
    }
}

pub(crate) fn post_load(l: State) {
    for metatable in META_TABLES {
        (metatable.post_gmod_open)(l);
    }
}

pub(crate) fn unload(_: State) {
    unsafe {
        LUA_STRUCTS.assume_init_drop();
    }
}

#[inline(always)]
fn get_map() -> &'static mut HashMap<*mut c_void, RStructInner> {
    unsafe { LUA_STRUCTS.assume_init_mut() }
}

pub(crate) fn push_struct<T: RStruct>(l: State, rstruct: T) {
    l.create_table(0, 0);

    // we create a new userdata with size of 0 because we are storing all objects in the userdata map
    // if lua allowed __gc on lightuserdata, we could use a lightuserdata instead
    let ud_ptr = l.new_userdata(0);
    l.create_table(0, 1);
    {
        l.push_function(T::internal__gc);
        l.set_field(-2, c"__gc");
    }
    l.set_metatable(-2);
    l.raw_seti(-2, INDEX_KEY);

    // set the metatable
    l.get_metatable_name(rstruct.metatable_name());
    l.set_metatable(-2);

    let boxed_struct = Box::new(rstruct);
    get_map().insert(
        ud_ptr,
        RStructInner {
            ptr: Box::into_raw(boxed_struct) as usize,
            ref_idx: Weak::new(),
        },
    );
}

pub(crate) fn get_struct<'a, T: RStruct>(l: State, idx: i32) -> Result<&'a mut T> {
    l.check_table(idx)?;
    l.raw_geti(idx, INDEX_KEY);

    let ud_ptr = l.to_userdata(-1);
    let str_inner = match get_map().get_mut(&ud_ptr) {
        Some(info) => info,
        None => bail!(
            "expected a userdata of type: {}",
            std::any::type_name::<T>()
        ),
    };

    unsafe {
        let ptr = str_inner.ptr as *mut T;
        Ok(&mut *ptr)
    }
}

pub(crate) fn get_struct_with_ref<'a, T: RStruct>(
    l: State,
    idx: i32,
) -> Result<(&'a mut T, LuaReference)> {
    l.check_table(idx)?;
    l.raw_geti(idx, INDEX_KEY);

    let ud_ptr = l.to_userdata(-1);
    let str_inner = match get_map().get_mut(&ud_ptr) {
        Some(info) => info,
        None => bail!(
            "expected a userdata of type: {}",
            std::any::type_name::<T>()
        ),
    };

    // if a weak reference is already present, use that instead of creating a new one
    let struct_ref = match str_inner.ref_idx.upgrade() {
        Some(lref) => LuaReference::new_from_arc(lref),
        None => {
            l.push_value(-1); // push the userdata again to keep the reference, as l.reference pops from the stack
            let lref = l.reference();
            str_inner.ref_idx = lref.weak_ref(); // have a weak reference to be used to avoid creating a lot of references for the same userdata
            lref
        }
    };

    unsafe {
        let ptr = str_inner.ptr as *mut T;
        Ok((&mut *ptr, struct_ref))
    }
}

#[macro_export]
macro_rules! register_lua_rstruct {
    // Branch without the impl block.
    // Branch with the impl block provided after the arrow.
    ($struct_name:ident, $meta_name:expr, $methods:expr => { $($body:tt)* } ) => {
        ::gmod::paste::paste! {
            #[used]
            #[allow(non_upper_case_globals)]
            #[::gmod::linkme::distributed_slice(::gmod::lua::rstruct::META_TABLES)]
            #[linkme(crate = ::gmod::linkme)]
            static [<metatable_ $struct_name>]: ::gmod::lua::rstruct::StructMetaTable = ::gmod::lua::rstruct::StructMetaTable {
                name: $meta_name,
                methods: $methods,
                post_gmod_open: $struct_name::post_gmod_open,
            };
        }
        impl ::gmod::lua::rstruct::RStruct for $struct_name {
            fn metatable_name(&self) -> ::gmod::lua::LuaCStr<'static> { $meta_name }
            $($body)*
        }
    };
    ($struct_name:ident, $meta_name:expr, $methods:expr) => {
        register_lua_rstruct!($struct_name, $meta_name, $methods => { });
    };
}
