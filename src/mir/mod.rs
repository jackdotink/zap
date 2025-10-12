use std::fmt::Display;

use crate::shared::NumberKind;

mod builder;
mod client;
mod serdes;
mod server;

pub use client::client;
pub use server::server;

struct Block {
    instrs: Vec<Instr>,
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for instr in &self.instrs {
            write!(f, "{instr}")?;
        }

        Ok(())
    }
}

enum Instr {
    Export {
        path: String,
        expr: Expr,
    },

    Init {
        dst: Var,
        src: Expr,
    },

    Assign {
        dst: Var,
        src: Expr,
    },

    AssignIndex {
        dst: Var,
        idx: Expr,
        src: Expr,
    },

    Call {
        call: Expr,
    },

    Return {
        exprs: Vec<Expr>,
    },

    ForRange {
        dst: Var,
        start: Expr,
        end: Expr,
        block: Block,
    },

    ForTable {
        idx: Var,
        val: Var,
        table: Expr,
        block: Block,
    },

    Branch {
        cond: Expr,
        then_block: Block,
        else_block: Block,
    },

    Function {
        dst: Var,
        args: Vec<Var>,
        block: Block,
    },

    Check {
        buf: Var,
        pos: Var,
        len: Var,
        size: Expr,
    },

    Reserve {
        dst: Var,
        pos: Var,
        size: Expr,
    },

    WriteK {
        func: FuncK,
        buf: Var,
        pos: Var,
        src: Expr,
    },

    WriteD {
        func: FuncD,
        buf: Var,
        pos: Var,
        src: Expr,
        size: Expr,
    },

    WriteReservedK {
        func: FuncK,
        buf: Var,
        pos: Expr,
        src: Expr,
    },

    ReadK {
        func: FuncK,
        buf: Var,
        pos: Var,
        dst: Var,
    },

    ReadD {
        func: FuncD,
        buf: Var,
        pos: Var,
        size: Expr,
        dst: Var,
    },
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instr::Export { path, expr } => write!(f, "export{path} = {expr};")?,

            Instr::Init { dst, src } => write!(f, "local {dst} = {src};")?,

            Instr::Assign { dst, src } => write!(f, "{dst} = {src};")?,

            Instr::AssignIndex { dst, idx, src } => write!(f, "{dst}[{idx}] = {src};")?,

            Instr::Call { call } => write!(f, "{call};")?,

            Instr::Return { exprs } => {
                write!(f, "return ")?;

                let mut exprs = exprs.iter().peekable();
                while let Some(expr) = exprs.next() {
                    write!(f, "{expr}")?;

                    if exprs.peek().is_some() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, ";")?;
            }

            Instr::ForRange {
                dst,
                start,
                end,
                block,
            } => {
                write!(f, "for {dst} = {start}, {end} do {block} end;")?;
            }

            Instr::ForTable {
                idx,
                val,
                table,
                block,
            } => {
                write!(f, "for {idx}, {val} in {table} do {block} end;")?;
            }

            Instr::Branch {
                cond,
                then_block,
                else_block,
            } => {
                write!(f, "if {cond} then {then_block} else {else_block} end;")?;
            }

            Instr::Function { dst, args, block } => {
                write!(f, "local function {dst}(")?;

                let mut args = args.iter().peekable();
                while let Some(arg) = args.next() {
                    write!(f, "{arg}")?;

                    if args.peek().is_some() {
                        write!(f, ",")?;
                    }
                }

                write!(f, ") {block} end;")?;
            }

            Instr::Check {
                buf,
                pos,
                len,
                size,
            } => {
                write!(f, "if {pos} + {size} >= {len} then ")?;
                write!(f, "{buf}, {len} = resize({buf}, {pos}, {len}, {size})")?;
                write!(f, "end;")?;
            }

            Instr::Reserve { dst, pos, size } => {
                write!(f, "local {dst} = {pos};")?;
                write!(f, "{pos} += {size};")?;
            }

            Instr::WriteK {
                func,
                buf,
                pos,
                src,
            } => {
                write!(f, "buffer.write{func}({buf}, {pos}, {src});")?;
                write!(f, "{pos} += {};", func.size())?;
            }

            Instr::WriteD {
                func,
                buf,
                pos,
                src,
                size,
            } => match func {
                FuncD::String => {
                    write!(f, "buffer.writestring({buf}, {pos}, {src}, {size});")?;
                    write!(f, "{pos} += {size};")?;
                }
            },

            Instr::WriteReservedK {
                func,
                buf,
                pos,
                src,
            } => {
                write!(f, "buffer.write{func}({buf}, {pos}, {src});")?;
            }

            Instr::ReadK {
                func,
                buf,
                pos,
                dst,
            } => {
                write!(f, "local {dst} = buffer.read{func}({buf}, {pos});")?;
                write!(f, "{pos} += {};", func.size())?;
            }

            Instr::ReadD {
                func,
                buf,
                pos,
                size,
                dst,
            } => match func {
                FuncD::String => {
                    write!(f, "local {dst} = buffer.readstring({buf}, {pos}, {size});")?;
                }
            },
        }

        Ok(())
    }
}

enum FuncK {
    U8,
    U16,
    U32,

    I8,
    I16,
    I32,

    F32,
    F64,
}

impl FuncK {
    fn size(&self) -> u32 {
        match self {
            FuncK::U8 | FuncK::I8 => 1,
            FuncK::U16 | FuncK::I16 => 2,
            FuncK::U32 | FuncK::I32 | FuncK::F32 => 4,
            FuncK::F64 => 8,
        }
    }
}

impl From<NumberKind> for FuncK {
    fn from(value: NumberKind) -> Self {
        match value {
            NumberKind::U8 => FuncK::U8,
            NumberKind::U16 => FuncK::U16,
            NumberKind::U24 => FuncK::U32,
            NumberKind::U32 => FuncK::U32,

            NumberKind::I8 => FuncK::I8,
            NumberKind::I16 => FuncK::I16,
            NumberKind::I24 => FuncK::I32,
            NumberKind::I32 => FuncK::I32,

            NumberKind::F32 | NumberKind::NaNF32 => FuncK::F32,
            NumberKind::F64 | NumberKind::NaNF64 => FuncK::F64,
        }
    }
}

impl Display for FuncK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FuncK::U8 => write!(f, "u8"),
            FuncK::U16 => write!(f, "u16"),
            FuncK::U32 => write!(f, "u32"),
            FuncK::I8 => write!(f, "i8"),
            FuncK::I16 => write!(f, "i16"),
            FuncK::I32 => write!(f, "i32"),
            FuncK::F32 => write!(f, "f32"),
            FuncK::F64 => write!(f, "f64"),
        }
    }
}

enum FuncD {
    String,
}

#[derive(Clone)]
enum Expr {
    Nil,

    Global(&'static str),

    Boolean(bool),
    Number(f64),
    String(String),

    Table(Vec<(Expr, Expr)>),

    Var(Var),

    Index(Box<Expr>, Box<Expr>),

    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnOp, Box<Expr>),

    Call(Box<Expr>, Vec<Expr>),
    Namecall(Box<Expr>, &'static str, Vec<Expr>),
}

impl From<bool> for Expr {
    fn from(value: bool) -> Self {
        Expr::Boolean(value)
    }
}

impl From<f64> for Expr {
    fn from(value: f64) -> Self {
        Expr::Number(value)
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        Expr::Number(value as f64)
    }
}

impl From<u32> for Expr {
    fn from(value: u32) -> Self {
        Expr::Number(value as f64)
    }
}

impl From<usize> for Expr {
    fn from(value: usize) -> Self {
        Expr::Number(value as f64)
    }
}

impl From<String> for Expr {
    fn from(value: String) -> Self {
        Expr::String(value)
    }
}

impl From<&str> for Expr {
    fn from(value: &str) -> Self {
        Expr::String(value.to_string())
    }
}

impl From<Vec<(Expr, Expr)>> for Expr {
    fn from(value: Vec<(Expr, Expr)>) -> Self {
        Expr::Table(value)
    }
}

impl From<Var> for Expr {
    fn from(value: Var) -> Self {
        Expr::Var(value)
    }
}

impl Expr {
    fn index(self, index: impl Into<Expr>) -> Expr {
        Expr::Index(Box::new(self), Box::new(index.into()))
    }

    fn and(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::And, Box::new(rhs.into()))
    }

    fn or(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Or, Box::new(rhs.into()))
    }

    fn add(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Add, Box::new(rhs.into()))
    }

    fn mul(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Mul, Box::new(rhs.into()))
    }

    fn mud(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Mod, Box::new(rhs.into()))
    }

    fn eq(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Eq, Box::new(rhs.into()))
    }

    fn lt(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Lt, Box::new(rhs.into()))
    }

    fn gt(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Gt, Box::new(rhs.into()))
    }

    fn le(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Le, Box::new(rhs.into()))
    }

    fn ge(self, rhs: impl Into<Expr>) -> Expr {
        Expr::Binary(Box::new(self), BinOp::Ge, Box::new(rhs.into()))
    }

    fn not(self) -> Expr {
        Expr::Unary(UnOp::Not, Box::new(self))
    }

    fn len(self) -> Expr {
        Expr::Unary(UnOp::Len, Box::new(self))
    }

    fn call(self, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(self), args)
    }

    fn namecall(self, method: &'static str, args: Vec<Expr>) -> Expr {
        Expr::Namecall(Box::new(self), method, args)
    }

    fn vector(x: impl Into<Expr>, y: impl Into<Expr>, z: impl Into<Expr>) -> Expr {
        Expr::Global("vector.create").call(vec![x.into(), y.into(), z.into()])
    }

    fn array(len: impl Into<Expr>) -> Expr {
        let len = len.into();
        if let Expr::Number(len) = len {
            let fields = (1..=(len as u32))
                .map(|i| (Expr::Number(i as f64), Expr::Nil))
                .collect::<Vec<_>>();

            Expr::Table(fields)
        } else {
            Expr::Global("table.create").call(vec![len])
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Nil => write!(f, "nil"),

            Expr::Global(name) => write!(f, "{name}"),

            Expr::Boolean(b) => write!(f, "{b}"),
            Expr::Number(n) => write!(f, "{n}"),
            Expr::String(s) => write!(f, "\"{}\"", s.as_bytes().escape_ascii()),

            Expr::Table(fields) => {
                let fields = fields
                    .iter()
                    .map(|(i, v)| format!("[{i}] = {v}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "{{ {fields} }}")
            }

            Expr::Var(v) => write!(f, "{v}"),

            Expr::Index(table, index) => write!(f, "{table}[{index}]"),

            Expr::Binary(lhs, op, rhs) => write!(f, "({lhs} {op} {rhs})"),
            Expr::Unary(op, expr) => write!(f, "({op}{expr})"),

            Expr::Call(func, args) => {
                let args = args
                    .iter()
                    .map(|a| format!("{a}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "{func}({args})")
            }

            Expr::Namecall(obj, method, args) => {
                let args = args
                    .iter()
                    .map(|a| format!("{a}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "{obj}:{method}({args})")
            }
        }
    }
}

#[derive(Clone, Copy)]
enum BinOp {
    And,
    Or,

    Add,
    Mul,
    Mod,

    Eq,
    Lt,
    Gt,
    Le,
    Ge,
}

impl Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::And => write!(f, "and"),
            BinOp::Or => write!(f, "or"),
            BinOp::Add => write!(f, "+"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Mod => write!(f, "%"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::Le => write!(f, "<="),
            BinOp::Ge => write!(f, ">="),
        }
    }
}

#[derive(Clone, Copy)]
enum UnOp {
    Not,
    Len,
}

impl Display for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnOp::Not => write!(f, "not "),
            UnOp::Len => write!(f, "#"),
        }
    }
}

#[derive(Clone, Copy)]
struct Var {
    pub id: u16,
}

impl Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.id)
    }
}
