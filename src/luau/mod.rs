use std::ptr::NonNull;

mod ffi;

struct Luau(NonNull<ffi::lua_State>);

impl Luau {}
