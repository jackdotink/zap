use std::ops::ControlFlow;

use lu::Methods;

use crate::{
    options,
    shared::{ApiCheck, Casing, NetworkSide, NumberKind, Range, Remote},
};

#[derive(Default)]
struct Config;

impl lu::Config for Config {
    type Allocator = lu::DefaultAllocator;

    type MainData = ();
    type ThreadData = ();
}

type Context = lu::Context<Config>;

pub fn exec(source: &[u8]) -> Result<Table, String> {
    let compiler = lu::Compiler::default();
    let bytecode = compiler.compile(source);

    let mut state = lu::State::new((), lu::DefaultAllocator);
    state.open_std();
    state.open_userdata::<Type>(Methods::default());
    state.open_userdata::<Event>(Methods::default());
    state.open_userdata::<OptionUserdata>(Methods::default());
    state.open_userdata::<RemoteUserdata>(Methods::default().with_method(
        "event",
        lu::Function::norm("RemoteUserdata::event", RemoteUserdata::event),
    ));
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

            Err(format!("error: {error}\ntrace: {trace}"))
        },

        lu::Status::Yield => unsafe {
            let trace = std::ffi::CStr::from_ptr(lu::sys::lua_debugtrace(thread.as_ptr()))
                .to_str()
                .unwrap();

            Err(format!("yielded: {trace}"))
        },

        _ => table(stack),
    }
}

#[derive(Clone)]
pub enum Item {
    Table(Table),
    Event(Event),
}

fn table(stack: &lu::Stack<Config>) -> Result<Table, String> {
    let mut options = options::Node::from(options::Partial::default());
    let mut items = Vec::new();

    let result = stack.iter(-1, || {
        if stack.to_number(-2).is_some_and(|n| n == 1.0) {
            options = match stack.to_userdata::<OptionUserdata>(-1) {
                Some(options) => options.borrow().0.clone(),
                None => return ControlFlow::Break("index 1 may only contain options".to_string()),
            };
        } else {
            let Some(name) = stack.to_string_str(-2) else {
                return ControlFlow::Break("item name must be a string".to_string());
            };

            match stack.type_of(-1) {
                lu::Type::Table => {
                    let table = match table(stack) {
                        Ok(table) => table,
                        Err(err) => return ControlFlow::Break(err),
                    };

                    table.opts.set_parent(options.clone());
                    items.push((name.to_owned(), Item::Table(table)));
                }

                lu::Type::Userdata => {
                    let Some(event) = stack.to_userdata::<Event>(-1) else {
                        return ControlFlow::Break(format!("item {name} must be a table or event"));
                    };

                    items.push((name.to_owned(), Item::Event(event.borrow().clone())));
                }

                _ => {
                    return ControlFlow::Break(format!("item {name} must be a table or event"));
                }
            }
        }

        ControlFlow::Continue(())
    });

    if let Some(err) = result {
        Err(err)
    } else {
        Ok(Table {
            opts: options,
            items,
        })
    }
}

fn library() -> lu::Library<Config> {
    let string = lu::Library::default()
        .with_function_norm("binary", binary_string)
        .with_function_norm("utf8", utf8_string);

    lu::Library::default()
        .with_function_norm("options", options)
        .with_function_norm("remote", remote)
        .with_function_norm("boolean", boolean)
        .with_function_norm("u8", u8)
        .with_function_norm("u16", u16)
        .with_function_norm("u24", u24)
        .with_function_norm("u32", u32)
        .with_function_norm("i8", i8)
        .with_function_norm("i16", i16)
        .with_function_norm("i24", i24)
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
        .with_function_norm("enum", enumn)
        .with_function_norm("struct", strukt)
}

#[derive(Clone)]
pub struct Table {
    pub opts: options::Node,
    pub items: Vec<(String, Item)>,
}

#[derive(lu::Userdata)]
pub struct OptionUserdata(options::Node);

extern "C-unwind" fn options(ctx: Context) -> lu::FnReturn {
    let mut options = options::Partial::default();
    ctx.arg_table(1);

    ctx.iter(1, || {
        match ctx.to_string_str(-2) {
            Some("apicheck") => {
                let value = match ctx.to_string_str(-1) {
                    Some("none") => ApiCheck::None,
                    Some("some") => ApiCheck::Some,
                    Some("full") => ApiCheck::Full,

                    _ => ctx.error_msg("apicheck must be 'none', 'some', or 'full'"),
                };

                options.apicheck = Some(value)
            }

            Some("casing") => {
                let value = match ctx.to_string_str(-1) {
                    Some("lower") => Casing::Lower,
                    Some("snake") => Casing::Snake,
                    Some("camel") => Casing::Camel,
                    Some("pascal") => Casing::Pascal,

                    _ => ctx.error_msg("casing must be 'lower', 'snake', 'camel', or 'pascal'"),
                };

                options.casing = Some(value);
            }

            Some(str) => ctx.error_msg(format!("unknown option: {str}")),
            None => ctx.error_msg("option names must be strings"),
        }

        ControlFlow::<()>::Continue(())
    });

    ctx.push_userdata(OptionUserdata(options::Node::from(options)));
    ctx.ret_with(1)
}

#[derive(lu::Userdata, Clone)]
pub struct Event {
    pub thru: Remote,
    pub from: NetworkSide,
    pub data: Vec<Type>,
}

#[derive(lu::Userdata)]
pub struct RemoteUserdata(Remote);

impl RemoteUserdata {
    extern "C-unwind" fn event(ctx: Context) -> lu::FnReturn {
        let remote = ctx.arg_userdata::<RemoteUserdata>(1);
        let from = ctx.arg_string_str(2);
        ctx.arg_table(3);

        let thru = remote.borrow().0.clone();

        let from = match from {
            "server" => NetworkSide::Server,
            "client" => NetworkSide::Client,

            _ => ctx.arg_error(2, c"must be 'server' or 'client'"),
        };

        let mut data = Vec::new();
        ctx.iter(3, || {
            if !ctx.is_number(-2) {
                ctx.error_msg("event data must be an array")
            }

            if let Some(item) = ctx.to_userdata::<Type>(-1) {
                data.push(item.borrow().clone());
            } else {
                ctx.error_msg("event data must be a type");
            }

            ControlFlow::<()>::Continue(())
        });

        ctx.push_userdata(Event { thru, from, data });
        ctx.ret_with(1)
    }
}

extern "C-unwind" fn remote(ctx: Context) -> lu::FnReturn {
    ctx.push_userdata(RemoteUserdata(Remote::default()));
    ctx.ret_with(1)
}

#[derive(lu::Userdata, Clone)]
pub enum Type {
    Boolean(BooleanType),
    Number(NumberType),
    Vector(VectorType),
    BinaryString(BinaryStringType),
    Utf8String(Utf8StringType),
    Array(ArrayType),
    Set(SetType),
    Map(MapType),
    Enum(EnumType),
    Struct(StructType),
}

#[derive(Clone)]
pub struct BooleanType;

extern "C-unwind" fn boolean(ctx: Context) -> lu::FnReturn {
    ctx.push_userdata(Type::Boolean(BooleanType));
    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct NumberType {
    pub kind: NumberKind,
    pub range: Range,
}

extern "C-unwind" fn u8(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(0f64..=255f64).contains(&min)
    {
        ctx.error_msg("u8 min must be between 0 and 255")
    }

    if let Some(max) = max
        && !(0f64..=255f64).contains(&max)
    {
        ctx.error_msg("u8 max must be between 0 and 255")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::U8,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn u16(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(0f64..=65535f64).contains(&min)
    {
        ctx.error_msg("u16 min must be between 0 and 65535")
    }

    if let Some(max) = max
        && !(0f64..=65535f64).contains(&max)
    {
        ctx.error_msg("u16 max must be between 0 and 65535")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::U16,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn u24(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(0f64..=16777215f64).contains(&min)
    {
        ctx.error_msg("u24 min must be between 0 and 16777215")
    }

    if let Some(max) = max
        && !(0f64..=16777215f64).contains(&max)
    {
        ctx.error_msg("u24 max must be between 0 and 16777215")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::U24,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn u32(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(0f64..=4294967295f64).contains(&min)
    {
        ctx.error_msg("u32 min must be between 0 and 4294967295")
    }

    if let Some(max) = max
        && !(0f64..=4294967295f64).contains(&max)
    {
        ctx.error_msg("u32 max must be between 0 and 4294967295")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::U32,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn i8(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(-128f64..=127f64).contains(&min)
    {
        ctx.error_msg("i8 min must be between -128 and 127")
    }

    if let Some(max) = max
        && !(-128f64..=127f64).contains(&max)
    {
        ctx.error_msg("i8 max must be between -128 and 127")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::I8,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn i16(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(-32768f64..=32767f64).contains(&min)
    {
        ctx.error_msg("i16 min must be between -32768 and 32767")
    }

    if let Some(max) = max
        && !(-32768f64..=32767f64).contains(&max)
    {
        ctx.error_msg("i16 max must be between -32768 and 32767")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::I16,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn i24(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(-8388608f64..=8388607f64).contains(&min)
    {
        ctx.error_msg("i24 min must be between -8388608 and 8388607")
    }

    if let Some(max) = max
        && !(-8388608f64..=8388607f64).contains(&max)
    {
        ctx.error_msg("i24 max must be between -8388608 and 8388607")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::I24,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn i32(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && !(-2147483648f64..=2147483647f64).contains(&min)
    {
        ctx.error_msg("i32 min must be between -2147483648 and 2147483647")
    }

    if let Some(max) = max
        && !(-2147483648f64..=2147483647f64).contains(&max)
    {
        ctx.error_msg("i32 max must be between -2147483648 and 2147483647")
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::I32,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn f32(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::F32,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn f64(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::F64,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn nanf32(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::NaNF32,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

extern "C-unwind" fn nanf64(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max")
    }

    ctx.push_userdata(Type::Number(NumberType {
        kind: NumberKind::NaNF64,
        range: Range { min, max },
    }));

    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct VectorType {
    pub x: NumberType,
    pub y: NumberType,
    pub z: Option<NumberType>,
}

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
                ctx.error_msg("vector components cannot be f64 or nanf64");
            }

            ctx.push_userdata(Type::Vector(VectorType { x, y, z: Some(z) }));
        }

        (Type::Number(x), Type::Number(y), None) => {
            if matches!(x.kind, NumberKind::F64 | NumberKind::NaNF64)
                || matches!(y.kind, NumberKind::F64 | NumberKind::NaNF64)
            {
                ctx.error_msg("vector components cannot be f64 or nanf64");
            }

            ctx.push_userdata(Type::Vector(VectorType { x, y, z: None }));
        }

        _ => ctx.error_msg("all vector components must be number types"),
    }

    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct BinaryStringType {
    pub len: Range,
}

extern "C-unwind" fn binary_string(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && min < 0f64
    {
        ctx.error_msg("length min cannot be negative");
    }

    if let Some(max) = max
        && max < 0f64
    {
        ctx.error_msg("length max cannot be negative");
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max");
    }

    ctx.push_userdata(Type::BinaryString(BinaryStringType {
        len: Range { min, max },
    }));

    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct Utf8StringType {
    pub len: Range,
}

extern "C-unwind" fn utf8_string(ctx: Context) -> lu::FnReturn {
    let min = ctx.arg_number_opt(1);
    let max = ctx.arg_number_opt(2);

    if let Some(min) = min
        && min < 0f64
    {
        ctx.error_msg("length min cannot be negative");
    }

    if let Some(max) = max
        && max < 0f64
    {
        ctx.error_msg("length max cannot be negative");
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max");
    }

    ctx.push_userdata(Type::Utf8String(Utf8StringType {
        len: Range { min, max },
    }));

    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct ArrayType {
    pub len: Range,
    pub item: Box<Type>,
}

extern "C-unwind" fn array(ctx: Context) -> lu::FnReturn {
    let item = ctx.arg_userdata::<Type>(1).borrow().clone();
    let min = ctx.arg_number_opt(2);
    let max = ctx.arg_number_opt(3);

    if let Some(min) = min
        && min < 0f64
    {
        ctx.error_msg("length min cannot be negative");
    }

    if let Some(max) = max
        && max < 0f64
    {
        ctx.error_msg("length max cannot be negative");
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max");
    }

    ctx.push_userdata(Type::Array(ArrayType {
        len: Range { min, max },
        item: Box::new(item),
    }));

    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct SetType {
    pub len: Range,
    pub item: Box<Type>,
}

extern "C-unwind" fn set(ctx: Context) -> lu::FnReturn {
    let item = ctx.arg_userdata::<Type>(1).borrow().clone();
    let min = ctx.arg_number_opt(2);
    let max = ctx.arg_number_opt(3);

    if let Some(min) = min
        && min < 0f64
    {
        ctx.error_msg("length min cannot be negative");
    }

    if let Some(max) = max
        && max < 0f64
    {
        ctx.error_msg("length max cannot be negative");
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max");
    }

    ctx.push_userdata(Type::Set(SetType {
        len: Range { min, max },
        item: Box::new(item),
    }));

    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct MapType {
    pub len: Range,
    pub index: Box<Type>,
    pub value: Box<Type>,
}

extern "C-unwind" fn map(ctx: Context) -> lu::FnReturn {
    let index = ctx.arg_userdata::<Type>(1).borrow().clone();
    let value = ctx.arg_userdata::<Type>(2).borrow().clone();
    let min = ctx.arg_number_opt(3);
    let max = ctx.arg_number_opt(4);

    if let Some(min) = min
        && min < 0f64
    {
        ctx.error_msg("length min cannot be negative");
    }

    if let Some(max) = max
        && max < 0f64
    {
        ctx.error_msg("length max cannot be negative");
    }

    if let Some(min) = min
        && let Some(max) = max
        && min > max
    {
        ctx.error_msg("min must be less than or equal to max");
    }

    ctx.push_userdata(Type::Map(MapType {
        len: Range { min, max },
        index: Box::new(index),
        value: Box::new(value),
    }));

    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct EnumType {
    pub variants: Vec<String>,
}

extern "C-unwind" fn enumn(ctx: Context) -> lu::FnReturn {
    ctx.arg_table(1);
    let mut variants = Vec::new();

    ctx.iter(1, || {
        let Some(str) = ctx.to_string_str(-1) else {
            ctx.error_msg("enum variant must be a string");
        };

        variants.push(str.to_string());
        ControlFlow::<()>::Continue(())
    });

    ctx.push_userdata(Type::Enum(EnumType { variants }));
    ctx.ret_with(1)
}

#[derive(Clone)]
pub struct StructType {
    pub fields: Vec<(String, Type)>,
}

extern "C-unwind" fn strukt(ctx: Context) -> lu::FnReturn {
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

        fields.push((field.unwrap().to_string(), value.unwrap()));
        ControlFlow::<()>::Continue(())
    });

    ctx.push_userdata(Type::Struct(StructType { fields }));
    ctx.ret_with(1)
}
