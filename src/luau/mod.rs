use std::{ffi::CStr, fmt::Display, ptr::NonNull};

use crate::{
    luau::compiler::{Bytecode, compile},
    types,
};

mod ffi;

mod api;
mod compiler;

pub fn run(source: &[u8]) -> Result<types::Type, String> {
    let luau = Luau::new();

    extern "C-unwind" fn error_handler(luau: Luau) -> libc::c_int {
        let err = luau.to_string_str(-1).unwrap_or("unknown error");
        let trace = unsafe {
            let ptr = ffi::lua_debugtrace(luau.as_ptr());
            std::str::from_utf8_unchecked(std::ffi::CStr::from_ptr(ptr).to_bytes())
        };

        luau.push_string(format!("{err}\ntraceback:\n{trace}"));

        1
    }

    luau.push_function(c"error_handler", error_handler);
    luau.push_bytecode(c"main", &compile(source));
    let status = luau.pcall(0, 1, -2);

    if status != Status::Ok {
        Err(luau
            .to_string_str(-1)
            .unwrap_or("unknown error")
            .to_string())
    } else {
        let result = luau
            .to_userdata::<types::Type>(-1)
            .ok_or("expected Type result from module")?;

        unsafe {
            ffi::lua_close(luau.as_ptr());
        }

        Ok(result.clone())
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok = ffi::LUA_OK as isize,
    Yield = ffi::LUA_YIELD as isize,
    ErrRuntime = ffi::LUA_ERRRUN as isize,
    ErrSyntax = ffi::LUA_ERRSYNTAX as isize,
    ErrMemory = ffi::LUA_ERRMEM as isize,
    ErrError = ffi::LUA_ERRERR as isize,
    Break = ffi::LUA_BREAK as isize,
}

impl From<ffi::lua_Status> for Status {
    fn from(value: ffi::lua_Status) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
    None = ffi::LUA_TNONE as isize,
    Nil = ffi::LUA_TNIL as isize,
    Boolean = ffi::LUA_TBOOLEAN as isize,
    LightUserdata = ffi::LUA_TLIGHTUSERDATA as isize,
    Number = ffi::LUA_TNUMBER as isize,
    Vector = ffi::LUA_TVECTOR as isize,
    String = ffi::LUA_TSTRING as isize,
    Table = ffi::LUA_TTABLE as isize,
    Function = ffi::LUA_TFUNCTION as isize,
    Userdata = ffi::LUA_TUSERDATA as isize,
    Thread = ffi::LUA_TTHREAD as isize,
    Buffer = ffi::LUA_TBUFFER as isize,
}

impl From<ffi::lua_Type> for Type {
    fn from(value: ffi::lua_Type) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::None => "no value",
            Self::Nil => "nil",
            Self::Boolean => "boolean",
            Self::LightUserdata => "userdata",
            Self::Number => "number",
            Self::Vector => "vector",
            Self::String => "string",
            Self::Table => "table",
            Self::Function => "function",
            Self::Userdata => "userdata",
            Self::Thread => "thread",
            Self::Buffer => "buffer",
        };

        write!(f, "{}", name)
    }
}

trait Userdata {
    fn tag() -> u8;
    fn name() -> &'static str;
    fn register(luau: &Luau);
}

#[repr(transparent)]
struct Luau(NonNull<ffi::lua_State>);

impl Luau {
    pub fn new() -> Self {
        #[allow(unused)]
        extern "C-unwind" fn lua_alloc(
            ud: *mut libc::c_void,
            ptr: *mut libc::c_void,
            osize: usize,
            nsize: usize,
        ) -> *mut libc::c_void {
            if nsize == 0 {
                unsafe { libc::free(ptr) };
                std::ptr::null_mut()
            } else {
                unsafe { libc::realloc(ptr, nsize) }
            }
        }

        let state = NonNull::new(unsafe { ffi::lua_newstate(lua_alloc, std::ptr::null_mut()) })
            .expect("failed to create lua state");

        unsafe {
            // only coroutine library is unavailable bc there is no usecase for
            // yielding and I don't want to deal with the complexity of multiple
            // threads
            ffi::luaopen_base(state.as_ptr());
            ffi::luaopen_table(state.as_ptr());
            ffi::luaopen_os(state.as_ptr());
            ffi::luaopen_string(state.as_ptr());
            ffi::luaopen_bit32(state.as_ptr());
            ffi::luaopen_buffer(state.as_ptr());
            ffi::luaopen_utf8(state.as_ptr());
            ffi::luaopen_math(state.as_ptr());
            ffi::luaopen_debug(state.as_ptr());
            ffi::luaopen_vector(state.as_ptr());
        }

        api::open(&Self(state));

        Self(state)
    }

    pub fn as_ptr(&self) -> *mut ffi::lua_State {
        self.0.as_ptr()
    }

    pub fn pcall(&self, nargs: u32, nresults: u32, errfunc: i32) -> Status {
        unsafe {
            Status::from(ffi::lua_pcall(
                self.as_ptr(),
                nargs as _,
                nresults as _,
                errfunc as _,
            ))
        }
    }

    pub fn error(&self, msg: impl AsRef<[u8]>) -> ! {
        self.push_string(msg);
        unsafe { ffi::lua_error(self.as_ptr()) };
    }

    pub fn set_global(&self, name: &CStr) {
        unsafe { ffi::lua_setfield(self.as_ptr(), ffi::LUA_GLOBALSINDEX, name.as_ptr()) };
    }

    pub fn get_top(&self) -> u32 {
        unsafe { ffi::lua_gettop(self.as_ptr()) as _ }
    }

    pub fn set_top(&self, n: u32) {
        unsafe { ffi::lua_settop(self.as_ptr(), n as _) }
    }

    pub fn pop(&self, n: u32) {
        self.set_top(self.get_top() - n);
    }

    pub fn remove(&self, idx: i32) {
        unsafe { ffi::lua_remove(self.as_ptr(), idx as _) };
    }

    pub fn insert(&self, idx: i32) {
        unsafe { ffi::lua_insert(self.as_ptr(), idx as _) };
    }

    pub fn replace(&self, idx: i32) {
        unsafe { ffi::lua_replace(self.as_ptr(), idx as _) };
    }

    pub fn check(&self, n: u32) {
        let result = unsafe { ffi::lua_checkstack(self.as_ptr(), n as _) };
        assert!(result != 0, "stack overflow");
    }

    pub fn push_copy(&self, idx: i32) {
        unsafe { ffi::lua_pushvalue(self.as_ptr(), idx) };
    }

    pub fn push_nil(&self) {
        unsafe { ffi::lua_pushnil(self.as_ptr()) };
    }

    pub fn push_boolean(&self, b: bool) {
        unsafe { ffi::lua_pushboolean(self.as_ptr(), b as _) };
    }

    pub fn push_light_userdata(&self, p: *mut libc::c_void) {
        unsafe { ffi::lua_pushlightuserdatatagged(self.as_ptr(), p, 0) };
    }

    pub fn push_number(&self, n: f64) {
        unsafe { ffi::lua_pushnumber(self.as_ptr(), n) };
    }

    pub fn push_vector(&self, v: (f32, f32, f32)) {
        unsafe { ffi::lua_pushvector(self.as_ptr(), v.0, v.1, v.2) };
    }

    pub fn push_string(&self, s: impl AsRef<[u8]>) {
        let s = s.as_ref();
        unsafe { ffi::lua_pushlstring(self.as_ptr(), s.as_ptr() as _, s.len()) };
    }

    pub fn push_table(&self) {
        unsafe { ffi::lua_createtable(self.as_ptr(), 0, 0) };
    }

    pub fn push_table_with(&self, narr: u32, nrec: u32) {
        unsafe { ffi::lua_createtable(self.as_ptr(), narr as _, nrec as _) };
    }

    pub fn push_function(
        &self,
        name: &'static CStr,
        func: extern "C-unwind" fn(luau: Luau) -> libc::c_int,
    ) {
        unsafe {
            let func = std::mem::transmute::<
                extern "C-unwind" fn(luau: Luau) -> libc::c_int,
                extern "C-unwind" fn(*mut ffi::lua_State) -> libc::c_int,
            >(func);

            ffi::lua_pushcclosurek(self.as_ptr(), func, name.as_ptr() as _, 0, None);
        }
    }

    pub fn push_bytecode(&self, name: &CStr, bytecode: &Bytecode) {
        unsafe {
            ffi::luau_load(
                self.as_ptr(),
                name.as_ptr() as _,
                bytecode.ptr() as _,
                bytecode.len() as _,
                0,
            );
        }
    }

    pub fn push_userdata<T: Userdata>(&self, ud: T) {
        let tag = T::tag();

        unsafe {
            let ptr = ffi::lua_newuserdatatagged(self.as_ptr(), size_of::<T>(), tag as _);
            let ptr = ptr as *mut T;

            ptr.write(ud);
        }
    }

    pub fn type_of(&self, idx: i32) -> Type {
        unsafe { Type::from(ffi::lua_type(self.as_ptr(), idx as _)) }
    }

    pub fn is_none(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::None
    }

    pub fn is_nil(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::Nil
    }

    pub fn is_boolean(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::Boolean
    }

    pub fn is_light_userdata(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::LightUserdata
    }

    pub fn is_number(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::Number
    }

    pub fn is_vector(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::Vector
    }

    pub fn is_string(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::String
    }

    pub fn is_table(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::Table
    }

    pub fn is_function(&self, idx: i32) -> bool {
        self.type_of(idx) == Type::Function
    }

    pub fn is_userdata<T: Userdata>(&self, idx: i32) -> bool {
        unsafe { ffi::lua_userdatatag(self.as_ptr(), idx) == (T::tag() as libc::c_int) }
    }

    pub fn to_boolean(&self, idx: i32) -> Option<bool> {
        if self.is_boolean(idx) {
            Some(unsafe { ffi::lua_toboolean(self.as_ptr(), idx as _) != 0 })
        } else {
            None
        }
    }

    pub fn to_light_userdata(&self, idx: i32) -> Option<*mut libc::c_void> {
        if self.is_light_userdata(idx) {
            Some(unsafe { ffi::lua_tolightuserdata(self.as_ptr(), idx as _) })
        } else {
            None
        }
    }

    pub fn to_number(&self, idx: i32) -> Option<f64> {
        let mut isnum = 0;
        let num = unsafe { ffi::lua_tonumberx(self.as_ptr(), idx, &mut isnum) };

        if isnum != 0 { Some(num) } else { None }
    }

    pub fn to_vector(&self, idx: i32) -> Option<(f32, f32, f32)> {
        let ptr = unsafe { ffi::lua_tovector(self.as_ptr(), idx) };

        if !ptr.is_null() {
            Some(unsafe { (ptr.read(), ptr.add(1).read(), ptr.add(2).read()) })
        } else {
            None
        }
    }

    pub fn to_string_slice(&self, idx: i32) -> Option<&[u8]> {
        let mut len = 0;
        let ptr = unsafe { ffi::lua_tolstring(self.as_ptr(), idx, &mut len) };

        if !ptr.is_null() {
            Some(unsafe { std::slice::from_raw_parts(ptr as _, len) })
        } else {
            None
        }
    }

    pub fn to_string_str(&self, idx: i32) -> Option<&str> {
        std::str::from_utf8(self.to_string_slice(idx)?).ok()
    }

    pub fn to_userdata<T: Userdata>(&self, idx: i32) -> Option<&mut T> {
        unsafe {
            ffi::lua_touserdatatagged(self.as_ptr(), idx, T::tag() as _)
                .cast::<T>()
                .as_mut()
        }
    }

    pub fn arg_boolean(&self, idx: u32) -> bool {
        if let Some(b) = self.to_boolean(idx as _) {
            b
        } else {
            self.error(format!(
                "bad argument #{idx} to function (boolean expected, got {})",
                self.type_of(idx as _)
            ))
        }
    }

    pub fn arg_boolean_opt(&self, idx: u32) -> Option<bool> {
        if let Some(b) = self.to_boolean(idx as _) {
            Some(b)
        } else {
            match self.type_of(idx as _) {
                Type::None | Type::Nil => None,

                ty => self.error(format!(
                    "bad argument #{idx} to function (boolean or nil expected, got {})",
                    ty
                )),
            }
        }
    }

    pub fn arg_number(&self, idx: u32) -> f64 {
        if let Some(n) = self.to_number(idx as _) {
            n
        } else {
            self.error(format!(
                "bad argument #{idx} to function (number expected, got {})",
                self.type_of(idx as _)
            ))
        }
    }

    pub fn arg_number_opt(&self, idx: u32) -> Option<f64> {
        if let Some(n) = self.to_number(idx as _) {
            Some(n)
        } else {
            match self.type_of(idx as _) {
                Type::None | Type::Nil => None,

                ty => self.error(format!(
                    "bad argument #{idx} to function (number or nil expected, got {})",
                    ty
                )),
            }
        }
    }

    pub fn arg_vector(&self, idx: u32) -> (f32, f32, f32) {
        if let Some(v) = self.to_vector(idx as _) {
            v
        } else {
            self.error(format!(
                "bad argument #{idx} to function (vector expected, got {})",
                self.type_of(idx as _)
            ))
        }
    }

    pub fn arg_vector_opt(&self, idx: u32) -> Option<(f32, f32, f32)> {
        if let Some(v) = self.to_vector(idx as _) {
            Some(v)
        } else {
            match self.type_of(idx as _) {
                Type::None | Type::Nil => None,

                ty => self.error(format!(
                    "bad argument #{idx} to function (vector or nil expected, got {})",
                    ty
                )),
            }
        }
    }

    pub fn arg_string_slice(&self, idx: u32) -> &[u8] {
        if let Some(s) = self.to_string_slice(idx as _) {
            s
        } else {
            self.error(format!(
                "bad argument #{idx} to function (string expected, got {})",
                self.type_of(idx as _)
            ))
        }
    }

    pub fn arg_string_str(&self, idx: u32) -> &str {
        if let Some(s) = self.to_string_str(idx as _) {
            s
        } else {
            self.error(format!(
                "bad argument #{idx} to function (utf8 string expected, got {})",
                self.type_of(idx as _)
            ))
        }
    }

    pub fn arg_string_opt_slice(&self, idx: u32) -> Option<&[u8]> {
        if let Some(s) = self.to_string_slice(idx as _) {
            Some(s)
        } else {
            match self.type_of(idx as _) {
                Type::None | Type::Nil => None,

                ty => self.error(format!(
                    "bad argument #{idx} to function (string or nil expected, got {})",
                    ty
                )),
            }
        }
    }

    pub fn arg_string_opt_str(&self, idx: u32) -> Option<&str> {
        if let Some(s) = self.to_string_str(idx as _) {
            Some(s)
        } else {
            match self.type_of(idx as _) {
                Type::None | Type::Nil => None,

                ty => self.error(format!(
                    "bad argument #{idx} to function (utf8 string or nil expected, got {})",
                    ty
                )),
            }
        }
    }

    pub fn arg_table(&self, idx: u32) {
        match self.type_of(idx as _) {
            Type::Table => (),

            ty => self.error(format!(
                "bad argument #{idx} to function (table expected, got {})",
                ty
            )),
        }
    }

    pub fn arg_table_opt(&self, idx: u32) -> Option<()> {
        match self.type_of(idx as _) {
            Type::Table => Some(()),
            Type::None | Type::Nil => None,

            ty => self.error(format!(
                "bad argument #{idx} to function (table or nil expected, got {})",
                ty
            )),
        }
    }

    pub fn arg_userdata<T: Userdata>(&self, idx: u32) -> &mut T {
        if let Some(ud) = self.to_userdata::<T>(idx as _) {
            ud
        } else {
            self.error(format!(
                "bad argument #{idx} to function ({} expected, got {})",
                T::name(),
                self.type_of(idx as _)
            ))
        }
    }

    pub fn arg_userdata_opt<T: Userdata>(&self, idx: u32) -> Option<&mut T> {
        if let Some(ud) = self.to_userdata::<T>(idx as _) {
            Some(ud)
        } else {
            match self.type_of(idx as _) {
                Type::None | Type::Nil => None,

                ty => self.error(format!(
                    "bad argument #{idx} to function ({} or nil expected, got {})",
                    T::name(),
                    ty
                )),
            }
        }
    }

    pub fn table_get(&self, tbl_idx: i32) {
        unsafe { ffi::lua_gettable(self.as_ptr(), tbl_idx as _) };
    }

    pub fn table_set(&self, tbl_idx: i32) {
        unsafe { ffi::lua_settable(self.as_ptr(), tbl_idx as _) };
    }

    pub fn table_get_field(&self, tbl_idx: i32, key: &CStr) {
        unsafe { ffi::lua_getfield(self.as_ptr(), tbl_idx as _, key.as_ptr() as _) };
    }

    pub fn table_set_field(&self, tbl_idx: i32, key: &CStr) {
        unsafe { ffi::lua_setfield(self.as_ptr(), tbl_idx as _, key.as_ptr() as _) };
    }

    pub fn table_get_raw(&self, tbl_idx: i32) {
        unsafe { ffi::lua_rawget(self.as_ptr(), tbl_idx as _) };
    }

    pub fn table_set_raw(&self, tbl_idx: i32) {
        unsafe { ffi::lua_rawset(self.as_ptr(), tbl_idx as _) };
    }

    pub fn table_get_raw_i(&self, tbl_idx: i32, key: u32) {
        unsafe { ffi::lua_rawgeti(self.as_ptr(), tbl_idx as _, key as _) };
    }

    pub fn table_set_raw_i(&self, tbl_idx: i32, key: u32) {
        unsafe { ffi::lua_rawseti(self.as_ptr(), tbl_idx as _, key as _) };
    }

    pub fn table_get_raw_field(&self, tbl_idx: i32, key: &CStr) {
        unsafe { ffi::lua_rawgetfield(self.as_ptr(), tbl_idx as _, key.as_ptr() as _) };
    }

    pub fn table_set_raw_field(&self, tbl_idx: i32, key: &CStr) {
        unsafe { ffi::lua_rawsetfield(self.as_ptr(), tbl_idx as _, key.as_ptr() as _) };
    }

    pub fn table_next(&self, tbl_idx: i32) -> bool {
        unsafe { ffi::lua_next(self.as_ptr(), tbl_idx as _) != 0 }
    }

    pub fn len(&self, idx: i32) -> usize {
        unsafe { ffi::lua_objlen(self.as_ptr(), idx as _) }
    }
}
