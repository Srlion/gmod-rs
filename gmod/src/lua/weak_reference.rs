use super::State;

// this is a reference to the weak lua meta table
static mut WEAK_REF_TABLE_INDEX: i32 = 0;

pub(crate) fn load(l: State) {
    // create the meta table with the __mode field set to "v"
    l.create_table(0, 1);
    {
        l.push_string("v");
        l.set_field(-2, c"__mode");
    }

    unsafe {
        WEAK_REF_TABLE_INDEX = l.raw_reference(); // save the reference to the weak lua meta table
    }
}

pub(crate) fn unload(_: State) {
    unsafe {
        WEAK_REF_TABLE_INDEX = 0;
    }
}

/// Pops the value off the stack, and returns a weak reference to it
///
/// Use `get_weak_ref` to get the weak reference from the registry table
pub(crate) fn weak_ref(l: State) -> i32 {
    // create a table and set it's metatable to the weak lua meta table
    l.create_table(1, 0);
    l.raw_getref(unsafe { WEAK_REF_TABLE_INDEX });
    l.set_metatable(-2); // set the metatable to be weak

    l.insert(-2); // move the table behind the value
    l.raw_seti(-2, 0); // set table[0] = value

    l.raw_reference()
}

/// Gets the weak reference from the registry table and pushes it to the stack
pub(crate) fn get_weak_ref(l: State, r#ref: i32) -> bool {
    l.raw_getref(r#ref);
    l.raw_geti(-1, 0);
    if l.is_nil(-1) {
        l.pop(); // pop the nil
        return false;
    }
    true
}
