use std::fmt::Display;

use crate::types::{NumberKind, Type};

mod build;

pub use build::Check;

pub fn build(ty: Type, checks: Check) -> Block {
    build::build(ty, checks)
}

#[derive(Debug, Clone)]
pub struct Block {
    pub instrs: Vec<Instr>,
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for instr in &self.instrs {
            write!(f, "{instr}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Instr {
    /// Assert that the given expression is truthy. If it is not, an error will
    /// be thrown.
    Assert { expr: Expr, msg: String },

    /// Allocate a fixed-size number of bytes. This should be used instead of
    /// `AllocD` when the size is known at compile time as it allows for the
    /// compiler to further optimize when the allocation takes place.
    AllocK { size: u32 },

    /// Allocate a dynamic number of bytes. This should be used when the size is
    /// dependent on runtime values, such as the length of an array.
    AllocD { size: Expr },

    /// Reserve a fixed-size number of bytes. Normally `WriteK` or `WriteD` will
    /// reserve and write to some memory, but this instruction can be used to
    /// reserve space without writing to it. This is useful for maps, where the
    /// length cannot be determined until the map is fully iterated, but the
    /// length must be written ahead of the values so that it can be decoded.
    ReserveK { into: Var, size: u32 },

    /// Write a value whose size is known at compile time.
    WriteK { func: FuncK, expr: Expr },

    /// Write a value whose size is not known at compile time.
    WriteD { func: FuncD, expr: Expr, size: Expr },

    /// Write a value whose size is known at compile time to a reserved
    /// location.
    WriteReservedK { func: FuncK, at: Expr, expr: Expr },

    /// Read a value whose size is known at compile time.
    ReadK { into: Var, func: FuncK },

    /// Read a value whose size is not known at compile time.
    ReadD { into: Var, func: FuncD, size: Expr },

    /// Initialize a variable with an expression.
    Expr { into: Var, expr: Expr },

    /// Assign to a variable.
    Assign { into: Var, expr: Expr },

    /// Assign to the index of a variable.
    AssignIndex { into: Var, index: Expr, expr: Expr },

    /// Iterate over a range.
    IterRange {
        into: Var,
        start: Expr,
        end: Expr,
        block: Block,
    },

    /// Iterate over a map.
    IterMap {
        index: Var,
        value: Var,
        map: Expr,
        block: Block,
    },
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instr::Assert { expr, msg } => {
                write!(f, "if not {expr} then error(\"{msg}\") end;")?;
            }

            Instr::AllocK { size } => {
                write!(f, "if pos + {size} > len then resize({size}) end;")?;
            }

            Instr::AllocD { size } => {
                write!(f, "if pos + {size} > len then resize({size}) end;")?;
            }

            Instr::ReserveK { into, size } => {
                write!(f, "local {into} = pos;")?;
                write!(f, "pos += {size};")?;
            }

            Instr::WriteK { func, expr } => {
                write!(f, "buffer.write{func}(buf, pos, {expr});")?;
                write!(f, "pos += {};", func.size())?;
            }

            Instr::WriteD { func, expr, size } => {
                write!(f, "buffer.write{func}(buf, pos, {expr}, {size});")?;
                write!(f, "pos += {size};")?;
            }

            Instr::WriteReservedK { func, at, expr } => {
                write!(f, "buffer.write{func}(buf, {at}, {expr});")?;
            }

            Instr::ReadK { into, func } => {
                write!(f, "local {into} = buffer.read{func}(buf, pos);")?;
                write!(f, "pos += {};", func.size())?;
            }

            Instr::ReadD { into, func, size } => {
                write!(f, "local {into} = buffer.read{func}(buf, pos, {size});")?;
                write!(f, "pos += {size};")?;
            }

            Instr::Expr { into, expr } => {
                write!(f, "local {into} = {expr};")?;
            }

            Instr::Assign { into, expr } => {
                write!(f, "{into} = {expr};")?;
            }

            Instr::AssignIndex { into, index, expr } => {
                write!(f, "{into}[{index}] = {expr};")?;
            }

            Instr::IterRange {
                into,
                start,
                end,
                block,
            } => {
                write!(f, "for {into} = {start}, {end} do {block} end;")?;
            }

            Instr::IterMap {
                index,
                value,
                map,
                block,
            } => {
                write!(f, "for {index}, {value} in {map} do {block} end;")?;
            }
        }

        Ok(())
    }
}

/// A writable type with a known size at compile time.
#[derive(Debug, Clone, Copy)]
pub enum FuncK {
    U8,
    U16,
    U32,

    I8,
    I16,
    I32,

    F32,
    F64,
}

impl From<NumberKind> for FuncK {
    fn from(value: NumberKind) -> Self {
        match value {
            NumberKind::U8 => FuncK::U8,
            NumberKind::U16 => FuncK::U16,
            NumberKind::U32 => FuncK::U32,

            NumberKind::I8 => FuncK::I8,
            NumberKind::I16 => FuncK::I16,
            NumberKind::I32 => FuncK::I32,

            NumberKind::F32 | NumberKind::NaNF32 => FuncK::F32,
            NumberKind::F64 | NumberKind::NaNF64 => FuncK::F64,
        }
    }
}

impl FuncK {
    fn size(&self) -> u32 {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 => 4,
            Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

impl Display for FuncK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::U8 => write!(f, "u8"),
            Self::U16 => write!(f, "u16"),
            Self::U32 => write!(f, "u32"),

            Self::I8 => write!(f, "i8"),
            Self::I16 => write!(f, "i16"),
            Self::I32 => write!(f, "i32"),

            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
        }
    }
}

/// A writable type whose size is not known at compile time.
#[derive(Debug, Clone, Copy)]
pub enum FuncD {
    String,
}

impl Display for FuncD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "string"),
        }
    }
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Root,

    Boolean(bool),
    Number(f64),
    String(String),

    Table,
    Array(Box<Expr>),
    Struct(Vec<(String, Expr)>),

    Var(Var),

    Index(Box<Expr>, Box<Expr>),

    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),

    Vector(Box<Expr>, Box<Expr>, Box<Expr>),
    Type(Box<Expr>),
    Utf8(Box<Expr>),
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

impl From<Var> for Expr {
    fn from(value: Var) -> Self {
        Expr::Var(value)
    }
}

impl Expr {
    fn index(self, index: impl Into<Expr>) -> Self {
        Expr::Index(Box::new(self), Box::new(index.into()))
    }

    fn and(self, other: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::And, Box::new(other.into()))
    }

    fn add(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Add, Box::new(rhs.into()))
    }

    fn mul(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Mul, Box::new(rhs.into()))
    }

    fn eq(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Eq, Box::new(rhs.into()))
    }

    fn lt(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Lt, Box::new(rhs.into()))
    }

    fn gt(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Gt, Box::new(rhs.into()))
    }

    fn le(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Le, Box::new(rhs.into()))
    }

    fn ge(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Ge, Box::new(rhs.into()))
    }

    fn len(self) -> Self {
        Expr::Unary(UnaryOp::Len, Box::new(self))
    }

    fn ty(self) -> Self {
        Expr::Type(Box::new(self))
    }

    fn utf8(self) -> Self {
        Expr::Utf8(Box::new(self))
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root => write!(f, "root"),

            Self::Boolean(b) => write!(f, "{b}"),
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "\"{}\"", s.escape_default()),

            Self::Table => write!(f, "{{}}"),
            Self::Array(expr) if matches!(**expr, Expr::Number(0.0)) => write!(f, "{{}}"),
            Self::Array(expr) => write!(f, "table.create({expr})"),
            Self::Struct(fields) => {
                write!(f, "{{ ")?;

                for (name, expr) in fields {
                    write!(f, "{name} = {expr}, ")?;
                }

                write!(f, "}}")
            }

            Self::Var(v) => write!(f, "{v}"),

            Self::Index(expr, index) => write!(f, "{expr}[{index}]"),

            Self::Binary(lhs, op, rhs) => write!(f, "({lhs} {op} {rhs})"),
            Self::Unary(op, expr) => write!(f, "({op}{expr})"),

            Self::Vector(x, y, z) => write!(f, "vector.create({x}, {y}, {z})"),
            Self::Type(expr) => write!(f, "type({expr})"),
            Self::Utf8(expr) => write!(f, "utf8.len({expr})"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    And,
    Add,
    Mul,
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::And => write!(f, "and"),
            Self::Add => write!(f, "+"),
            Self::Mul => write!(f, "*"),
            Self::Eq => write!(f, "=="),
            Self::Lt => write!(f, "<"),
            Self::Gt => write!(f, ">"),
            Self::Le => write!(f, "<="),
            Self::Ge => write!(f, ">="),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Len,
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Len => write!(f, "#"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Var(pub u16);

impl Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}
