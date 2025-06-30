use crate::types::{
    ArrayType, BinaryStringType, Event, EventFrom, MapType, NumberKind, NumberType, Range, SetType,
    StructType, Type, Utf8StringType, VectorType,
};

pub fn exec(code: &[u8]) -> Result<Type, String> {
    let compiler = lu::Compiler::default();
    let bytecode = compiler.compile(code);

    let mut state = lu::State::new((), lu::DefaultAllocator);
    state.open_std();
    state.open_userdata::<Type>();
    state.open_library("zap", library());
    state.sandbox();

    let thread = state.new_thread();
    let stack = thread.stack();

    stack.push_bytecode(c"main", bytecode.bytecode());

    match thread.resume(None, 0) {
        lu::Status::ErrorHander | lu::Status::ErrorMemory | lu::Status::ErrorRuntime => unsafe {
            let error = stack.to_string_str(-1).unwrap_or("non-string error");

            let trace = std::ffi::CStr::from_ptr(lu::sys::lua_debugtrace(thread.as_ptr()))
                .to_str()
                .unwrap();

            Err(format!("Error: {}\nTrace: {}", error, trace))
        },

        lu::Status::Yield => unsafe {
            let trace = std::ffi::CStr::from_ptr(lu::sys::lua_debugtrace(thread.as_ptr()))
                .to_str()
                .unwrap();

            Err(format!("Yielded: {}", trace))
        },

        _ => {
            let Some(ty) = stack.to_userdata::<Type>(-1) else {
                return Err("Expected a Type to be returned".to_string());
            };

            Ok(ty.borrow().clone())
        }
    }
}

#[derive(Default)]
struct Config;

impl lu::Config for Config {
    type Allocator = lu::DefaultAllocator;

    type MainData = ();
    type ThreadData = ();
}

fn library() -> lu::Library<Config> {
    let string = lu::Library::default()
        .with_function_norm("binary", string_binary)
        .with_function_norm("utf8", string_utf8);

    lu::Library::default()
        .with_function_norm("u8", u8)
        .with_function_norm("u16", u16)
        .with_function_norm("u32", u32)
        .with_function_norm("i8", i8)
        .with_function_norm("i16", i16)
        .with_function_norm("i32", i32)
        .with_function_norm("f32", f32)
        .with_function_norm("f64", f64)
        .with_function_norm("nanf32", nanf32)
        .with_function_norm("nanf64", nanf64)
        .with_function_norm("vector", vector)
        .with("string", string)
        .with_function_norm("array", array)
        .with_function_norm("set", set)
        .with_function_norm("map", map)
        .with_function_norm("struct", struckt)
        .with_function_norm("reliable", reliable_event)
        .with_function_norm("unreliable", unreliable_event)
}

type Context = lu::Context<Config>;

extern "C-unwind" fn reliable_event(ctx: Context) -> lu::FnReturn {
    let from = match ctx.arg_string_str(1) {
        "server" => EventFrom::Server,
        "client" => EventFrom::Client,

        _ => ctx.arg_error(1, c"expected 'server' or 'client'"),
    };

    ctx.arg_table(2);
    let mut data = Vec::new();

    ctx.iter(2, || {
        let ty = ctx.to_userdata::<Type>(-1).map(|u| u.borrow().clone());

        match ty {
            Some(ty) => data.push(ty),
            None => ctx.error_msg("event data must be a type"),
        }
    });

    ctx.push_userdata(Event {
        from,
        data,
        reliable: true,
    });

    ctx.ret_with(1)
}

extern "C-unwind" fn unreliable_event(ctx: Context) -> lu::FnReturn {
    let from = match ctx.arg_string_str(1) {
        "server" => EventFrom::Server,
        "client" => EventFrom::Client,

        _ => ctx.arg_error(1, c"expected 'server' or 'client'"),
    };

    ctx.arg_table(2);
    let mut data = Vec::new();

    ctx.iter(2, || {
        let ty = ctx.to_userdata::<Type>(-1).map(|u| u.borrow().clone());

        match ty {
            Some(ty) => data.push(ty),
            None => ctx.error_msg("event data must be a type"),
        }
    });

    ctx.push_userdata(Event {
        from,
        data,
        reliable: false,
    });

    ctx.ret_with(1)
}

macro_rules! number {
    ($ty:ty, $kind:ident, $name:ident) => {
        extern "C-unwind" fn $name(ctx: Context) -> lu::FnReturn {
            let min = ctx.arg_number_opt(1);
            let max = ctx.arg_number_opt(2);

            if let Some(min) = min {
                if !((<$ty>::MIN as f64) < min || min < (<$ty>::MAX as f64)) {
                    ctx.error_msg("range minimum out of bounds");
                }
            }

            if let Some(max) = max {
                if !((<$ty>::MIN as f64) < max || max < (<$ty>::MAX as f64)) {
                    ctx.error_msg("range maximum out of bounds");
                }
            }

            if let (Some(min), Some(max)) = (min, max) {
                if min > max {
                    ctx.error_msg("range minimum cannot be greater than maximum");
                }
            }

            ctx.push_userdata(Type::Number(NumberType {
                kind: NumberKind::$kind,
                range: Range { min, max },
            }));

            ctx.ret_with(1)
        }
    };
}

number!(u8, U8, u8);
number!(u16, U16, u16);
number!(u32, U32, u32);

number!(i8, I8, i8);
number!(i16, I16, i16);
number!(i32, I32, i32);

number!(f32, F32, f32);
number!(f64, F64, f64);
number!(f32, NaNF32, nanf32);
number!(f64, NaNF64, nanf64);

extern "C-unwind" fn vector(ctx: Context) -> lu::FnReturn {
    let x = ctx.arg_userdata::<Type>(1).borrow().clone();
    let y = ctx.arg_userdata::<Type>(2).borrow().clone();
    let z = ctx.arg_userdata_opt::<Type>(3).map(|z| z.borrow().clone());

    match (x, y, z) {
        (Type::Number(x), Type::Number(y), Some(Type::Number(z))) => {
            if matches!(x.kind, NumberKind::F64 | NumberKind::NaNF64)
                || matches!(y.kind, NumberKind::F64 | NumberKind::NaNF64)
                || matches!(z.kind, NumberKind::F64 | NumberKind::NaNF64)
            {
                ctx.error_msg("vector components cannot be f64 or NaNF64");
            }

            ctx.push_userdata(Type::Vector(VectorType { x, y, z: Some(z) }));
        }

        (Type::Number(x), Type::Number(y), None) => {
            if matches!(x.kind, NumberKind::F64 | NumberKind::NaNF64)
                || matches!(y.kind, NumberKind::F64 | NumberKind::NaNF64)
            {
                ctx.error_msg("vector components cannot be f64 or NaNF64");
            }

            ctx.push_userdata(Type::Vector(VectorType { x, y, z: None }));
        }

        _ => ctx.error_msg("all vector components must be number types"),
    }

    ctx.ret_with(1)
}

fn len_range(ctx: &Context, offset: u32) -> Range {
    let min = ctx.arg_number_opt(offset + 1);
    let max = ctx.arg_number_opt(offset + 2);

    if let Some(min) = min {
        if min < 0.0 {
            ctx.error_msg("length minimum cannot be negative");
        }
    }

    if let Some(max) = max {
        if max < 0.0 {
            ctx.error_msg("length maximum cannot be negative");
        }
    }

    if let (Some(min), Some(max)) = (min, max) {
        if min > max {
            ctx.error_msg("length minimum cannot be greater than maximum");
        }
    }

    Range { min, max }
}

extern "C-unwind" fn string_binary(ctx: Context) -> lu::FnReturn {
    let len = len_range(&ctx, 0);

    ctx.push_userdata(Type::BinaryString(BinaryStringType { len }));
    ctx.ret_with(1)
}

extern "C-unwind" fn string_utf8(ctx: Context) -> lu::FnReturn {
    let len = len_range(&ctx, 0);

    ctx.push_userdata(Type::Utf8String(Utf8StringType { len }));
    ctx.ret_with(1)
}

extern "C-unwind" fn array(ctx: Context) -> lu::FnReturn {
    let item = Box::new(ctx.arg_userdata::<Type>(1).borrow().clone());
    let len = len_range(&ctx, 1);

    ctx.push_userdata(Type::Array(ArrayType { len, item }));
    ctx.ret_with(1)
}

extern "C-unwind" fn set(ctx: Context) -> lu::FnReturn {
    let item = Box::new(ctx.arg_userdata::<Type>(1).borrow().clone());
    let len = len_range(&ctx, 1);

    ctx.push_userdata(Type::Set(SetType { len, item }));
    ctx.ret_with(1)
}

extern "C-unwind" fn map(ctx: Context) -> lu::FnReturn {
    let index = Box::new(ctx.arg_userdata::<Type>(1).borrow().clone());
    let value = Box::new(ctx.arg_userdata::<Type>(2).borrow().clone());
    let len = len_range(&ctx, 2);

    ctx.push_userdata(Type::Map(MapType { index, value, len }));
    ctx.ret_with(1)
}

extern "C-unwind" fn struckt(ctx: Context) -> lu::FnReturn {
    ctx.arg_table(1);
    let mut fields = Vec::new();

    ctx.iter(1, || {
        let field = ctx.to_string_str(-2);
        let value = ctx.to_userdata::<Type>(-1).map(|u| u.borrow().clone());

        if field.is_none() {
            ctx.error_msg("struct field name must be a string");
        } else if value.is_none() {
            ctx.error_msg("struct field value must be a type");
        }

        fields.push((field.unwrap().to_string(), value.unwrap()))
    });

    ctx.push_userdata(Type::Struct(StructType { fields }));
    ctx.ret_with(1)
}
