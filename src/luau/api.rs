use crate::{
    luau::{Luau, Userdata, ffi},
    types::{
        ArrayType, BinaryStringType, MapType, NumberKind, NumberType, Range, SetType, StructType,
        Type, Utf8StringType, VectorType,
    },
};

impl Userdata for Type {
    fn tag() -> u8 {
        1
    }

    fn name() -> &'static str {
        "Type"
    }

    fn register(luau: &Luau) {
        extern "C-unwind" fn cleanup(_: *mut ffi::lua_State, ud: *mut std::ffi::c_void) {
            unsafe { (ud as *mut Type).drop_in_place() };
        }

        luau.push_table();
        luau.push_string(Self::name());
        luau.table_set_raw_field(-2, c"__type");

        unsafe {
            ffi::lua_setuserdatadtor(luau.as_ptr(), Self::tag() as _, cleanup);
            ffi::lua_setuserdatametatable(luau.as_ptr(), Self::tag() as _);
        }
    }
}

macro_rules! number {
    ($ty:ty, $kind:ident, $name:ident) => {
        extern "C-unwind" fn $name(luau: Luau) -> libc::c_int {
            let min = luau.arg_number_opt(1);
            let max = luau.arg_number_opt(2);

            if let Some(min) = min {
                if !((<$ty>::MIN as f64) < min || min < (<$ty>::MAX as f64)) {
                    luau.error("range minimum out of bounds");
                }
            }

            if let Some(max) = max {
                if !((<$ty>::MIN as f64) < max || max < (<$ty>::MAX as f64)) {
                    luau.error("range maximum out of bounds");
                }
            }

            if let (Some(min), Some(max)) = (min, max) {
                if min > max {
                    luau.error("range minimum cannot be greater than maximum");
                }
            }

            luau.push_userdata(Type::Number(NumberType {
                kind: NumberKind::$kind,
                range: Range { min, max },
            }));

            1
        }
    };
}

number!(u8, U8, zap_u8);
number!(u16, U16, zap_u16);
number!(u32, U32, zap_u32);

number!(i8, I8, zap_i8);
number!(i16, I16, zap_i16);
number!(i32, I32, zap_i32);

number!(f32, F32, zap_f32);
number!(f64, F64, zap_f64);

number!(f32, NaNF32, zap_nan_f32);
number!(f64, NaNF64, zap_nan_f64);

extern "C-unwind" fn zap_vector(luau: Luau) -> libc::c_int {
    let x = luau.arg_userdata::<Type>(1);
    let y = luau.arg_userdata::<Type>(2);
    let z = luau.arg_userdata_opt::<Type>(3);

    match (x, y, z) {
        (Type::Number(x), Type::Number(y), Some(Type::Number(z))) => {
            if matches!(x.kind, NumberKind::F64 | NumberKind::NaNF64)
                || matches!(y.kind, NumberKind::F64 | NumberKind::NaNF64)
                || matches!(z.kind, NumberKind::F64 | NumberKind::NaNF64)
            {
                luau.error("vector components cannot be f64 or NaNF64");
            }

            luau.push_userdata(Type::Vector(VectorType {
                x: x.clone(),
                y: y.clone(),
                z: Some(z.clone()),
            }));
        }

        (Type::Number(x), Type::Number(y), None) => {
            if matches!(x.kind, NumberKind::F64 | NumberKind::NaNF64)
                || matches!(y.kind, NumberKind::F64 | NumberKind::NaNF64)
            {
                luau.error("vector components cannot be f64 or NaNF64");
            }

            luau.push_userdata(Type::Vector(VectorType {
                x: x.clone(),
                y: y.clone(),
                z: None,
            }));
        }

        _ => luau.error("all vector components must be number types"),
    }

    1
}

fn len_range(luau: &Luau, offset: u32) -> Range {
    let min = luau.arg_number_opt(offset + 1);
    let max = luau.arg_number_opt(offset + 2);

    if let Some(min) = min {
        if min < 0.0 {
            luau.error("length minimum cannot be negative");
        }
    }

    if let Some(max) = max {
        if max < 0.0 {
            luau.error("length maximum cannot be negative");
        }
    }

    if let (Some(min), Some(max)) = (min, max) {
        if min > max {
            luau.error("length minimum cannot be greater than maximum");
        }
    }

    Range { min, max }
}

extern "C-unwind" fn zap_string_binary(luau: Luau) -> libc::c_int {
    let len = len_range(&luau, 0);
    luau.push_userdata(Type::BinaryString(BinaryStringType { len }));

    1
}

extern "C-unwind" fn zap_string_utf8(luau: Luau) -> libc::c_int {
    let len = len_range(&luau, 0);
    luau.push_userdata(Type::Utf8String(Utf8StringType { len }));

    1
}

extern "C-unwind" fn zap_array(luau: Luau) -> libc::c_int {
    let item = luau.arg_userdata::<Type>(1);
    let len = len_range(&luau, 1);

    luau.push_userdata(Type::Array(ArrayType {
        item: Box::new(item.clone()),
        len,
    }));

    1
}

extern "C-unwind" fn zap_set(luau: Luau) -> libc::c_int {
    let item = luau.arg_userdata::<Type>(1);
    let len = len_range(&luau, 1);

    luau.push_userdata(Type::Set(SetType {
        item: Box::new(item.clone()),
        len,
    }));

    1
}

extern "C-unwind" fn zap_map(luau: Luau) -> libc::c_int {
    let index = luau.arg_userdata::<Type>(1);
    let value = luau.arg_userdata::<Type>(2);
    let len = len_range(&luau, 2);

    luau.push_userdata(Type::Map(MapType {
        index: Box::new(index.clone()),
        value: Box::new(value.clone()),
        len,
    }));

    1
}

extern "C-unwind" fn zap_struct(luau: Luau) -> libc::c_int {
    let mut fields = Vec::new();
    luau.arg_table(1);

    luau.push_nil();
    while luau.table_next(-2) {
        let field = luau.to_string_str(-2);
        let value = luau.to_userdata::<Type>(-1);

        if field.is_none() {
            luau.error("struct field names must be strings");
        } else if value.is_none() {
            luau.error("struct field values must be types");
        }

        fields.push((field.unwrap().to_string(), value.unwrap().clone()));
        luau.pop(1);
    }

    luau.push_userdata(Type::Struct(StructType { fields }));

    1
}

pub fn open(luau: &Luau) {
    Type::register(luau);

    luau.push_table();

    luau.push_function(c"zap_u8", zap_u8);
    luau.table_set_raw_field(-2, c"u8");

    luau.push_function(c"zap_u16", zap_u16);
    luau.table_set_raw_field(-2, c"u16");

    luau.push_function(c"zap_u32", zap_u32);
    luau.table_set_raw_field(-2, c"u32");

    luau.push_function(c"zap_i8", zap_i8);
    luau.table_set_raw_field(-2, c"i8");

    luau.push_function(c"zap_i16", zap_i16);
    luau.table_set_raw_field(-2, c"i16");

    luau.push_function(c"zap_i32", zap_i32);
    luau.table_set_raw_field(-2, c"i32");

    luau.push_function(c"zap_f32", zap_f32);
    luau.table_set_raw_field(-2, c"f32");

    luau.push_function(c"zap_f64", zap_f64);
    luau.table_set_raw_field(-2, c"f64");

    luau.push_function(c"zap_nan_f32", zap_nan_f32);
    luau.table_set_raw_field(-2, c"nanf32");

    luau.push_function(c"zap_nan_f64", zap_nan_f64);
    luau.table_set_raw_field(-2, c"nanf64");

    luau.push_function(c"zap_vector", zap_vector);
    luau.table_set_raw_field(-2, c"vector");

    luau.push_table();

    luau.push_function(c"zap_string_binary", zap_string_binary);
    luau.table_set_raw_field(-2, c"binary");

    luau.push_function(c"zap_string_utf8", zap_string_utf8);
    luau.table_set_raw_field(-2, c"utf8");

    luau.table_set_raw_field(-2, c"string");

    luau.push_function(c"zap_array", zap_array);
    luau.table_set_raw_field(-2, c"array");

    luau.push_function(c"zap_set", zap_set);
    luau.table_set_raw_field(-2, c"set");

    luau.push_function(c"zap_map", zap_map);
    luau.table_set_raw_field(-2, c"map");

    luau.push_function(c"zap_struct", zap_struct);
    luau.table_set_raw_field(-2, c"struct");

    luau.set_global(c"zap");
}
