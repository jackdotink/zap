use crate::luau::ffi;

pub fn compile(source: &[u8]) -> Bytecode {
    let mut len = 0;
    let ptr = unsafe {
        ffi::luau_compile(
            source.as_ptr() as _,
            source.len(),
            std::ptr::null_mut(),
            &mut len,
        ) as *const u8
    };

    Bytecode { ptr, len }
}

pub struct Bytecode {
    ptr: *const u8,
    len: usize,
}

impl Bytecode {
    pub fn ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
