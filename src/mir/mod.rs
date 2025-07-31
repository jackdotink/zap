use std::fmt::Display;

use crate::{hir, shared::NumberKind};

mod builder;
mod client;
mod serdes;
mod server;

pub fn server(table: &hir::Table) -> Result<String, std::fmt::Error> {
    use std::fmt::Write;

    let mut b = builder::Builder::default();
    let server = server::Server::default();

    server.table(&mut b, table);

    let mut s = String::new();

    writeln!(s, "{}", include_str!("../header.luau"))?;
    writeln!(s, "local result = {{}}")?;
    writeln!(s, "local folder = Instance.new('Folder')")?;
    writeln!(s, "folder.Name = 'Z'")?;
    writeln!(s, "folder.Parent = game.ReplicatedStorage")?;
    writeln!(s, "{}", b.build())?;
    writeln!(s, "return result")?;

    Ok(s)
}

pub fn client(table: &hir::Table) -> Result<String, std::fmt::Error> {
    use std::fmt::Write;

    let mut b = builder::Builder::default();
    let client = client::Client::default();

    client.table(&mut b, table);

    let mut s = String::new();

    writeln!(s, "{}", include_str!("../header.luau"))?;
    writeln!(s, "local result = {{}}")?;
    writeln!(s, "local folder = game.ReplicatedStorage:WaitForChild('Z')")?;
    writeln!(s, "{}", b.build())?;
    writeln!(s, "return result")?;

    Ok(s)
}

#[derive(Clone)]
pub struct Block {
    pub instrs: Vec<Instr>,
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for instr in self.instrs.iter() {
            write!(f, "{instr}")?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub enum Instr {
    /// An arbitrary Luau statement.
    Stmt { stmt: String },

    /// Assert that the given expression is truthy.
    Assert { expr: Expr, msg: String },

    /// Allocate a fixed-size number of bytes.
    AllocK { size: u32 },

    /// Allocate a dynamic number of bytes.
    AllocD { size: Expr },

    /// Reserve a fixed-size number of bytes.
    ReserveK { into: Var, size: u32 },

    /// Write a value of a fixed size.
    WriteK { func: FuncK, expr: Expr },

    /// Write a value of a dynamic size.
    WriteD { func: FuncD, expr: Expr, size: Expr },

    /// Write a value of a fixed size to a reserved location.
    WriteReservedK { func: FuncK, at: Expr, expr: Expr },

    /// Initialize a variable by reading a value of a fixed size.
    ReadK { into: Var, func: FuncK },

    /// Initialize a variable by reading a value of a dynamic size.
    ReadD { into: Var, func: FuncD, size: Expr },

    /// Initialize a variable with an expression.
    Expr { into: Var, expr: Expr },

    /// Assign a value to a variable.
    Assign { into: Var, expr: Expr },

    /// Assign a value to the index of a variable.
    AssignIndex { into: Var, index: Expr, expr: Expr },

    /// Iterate over an integer range.
    ForRange {
        into: Var,
        start: Expr,
        end: Expr,
        block: Block,
    },

    /// Iterate over the indices and values of a table.
    ForTable {
        index: Var,
        value: Var,
        table: Expr,
        block: Block,
    },

    /// Branch on a condition.
    Branch {
        cond: Expr,
        then_block: Block,
        else_block: Block,
    },

    /// Declare a function and initialize the variable with it.
    Function {
        into: Var,
        args: Vec<Var>,
        body: Block,
    },
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instr::Stmt { stmt } => write!(f, "{stmt};")?,

            Instr::Assert { expr, msg } => write!(f, "if not {expr} then error(\"{msg}\") end;")?,

            Instr::AllocK { size } => write!(f, "if pos + {size} > len then resize({size}) end;")?,

            Instr::AllocD { size } => write!(f, "if pos + {size} > len then resize({size}) end;")?,

            Instr::ReserveK { into, size } => {
                write!(f, "local {into} = pos;")?;
                write!(f, "pos += {size};")?;
            }

            Instr::WriteK { func, expr } => {
                write!(f, "buffer.write{func}(buf, pos, {expr});")?;
                write!(f, "pos += {};", func.size())?;
            }

            Instr::WriteD { func, expr, size } => {
                write!(f, "local size = {size};")?;
                write!(f, "buffer.write{func}(buf, pos, {expr}, size);")?;
                write!(f, "pos += size;")?;
            }

            Instr::WriteReservedK { func, at, expr } => {
                write!(f, "buffer.write{func}(buf, {at}, {expr});")?;
                write!(f, "pos += {};", func.size())?;
            }

            Instr::ReadK { into, func } => {
                write!(f, "local {into} = buffer.read{func}(buf, pos);")?;
                write!(f, "pos += {};", func.size())?;
            }

            Instr::ReadD { into, func, size } => {
                write!(f, "local size = {size};")?;
                write!(f, "local {into} = buffer.read{func}(buf, pos, size);")?;
                write!(f, "pos += size;")?;
            }

            Instr::Expr { into, expr } => write!(f, "local {into} = {expr};")?,

            Instr::Assign { into: var, expr } => write!(f, "{var} = {expr};")?,

            Instr::AssignIndex { into, index, expr } => write!(f, "{into}[{index}] = {expr};")?,

            Instr::ForRange {
                into,
                start,
                end,
                block,
            } => write!(f, "for {into} = {start}, {end} do {block} end;")?,

            Instr::ForTable {
                index,
                value,
                table,
                block,
            } => write!(f, "for {index}, {value} in {table} do {block} end;")?,

            Instr::Branch {
                cond,
                then_block,
                else_block,
            } => write!(f, "if {cond} then {then_block} else {else_block} end;")?,

            Instr::Function { into, args, body } => {
                let args = args
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "local function {into}({args}) {body} end;")?;
            }
        }

        Ok(())
    }
}

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
    pub fn size(&self) -> u32 {
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

#[derive(Clone)]
pub enum Expr {
    Boolean(bool),
    Number(f64),
    String(String),

    Table(Vec<(Expr, Expr)>),
    Array(Box<Expr>),

    Var(Var),

    Index(Box<Expr>, Box<Expr>),

    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),

    Vector(Box<Expr>, Box<Expr>, Box<Expr>),
    Type(Box<Expr>),
    Utf8(Box<Expr>),
    Bit(Box<Expr>),
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

impl From<u32> for Expr {
    fn from(value: u32) -> Self {
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

impl From<Var> for Expr {
    fn from(value: Var) -> Self {
        Expr::Var(value)
    }
}

impl Expr {
    pub fn index(self, index: impl Into<Expr>) -> Self {
        Expr::Index(Box::new(self), Box::new(index.into()))
    }

    pub fn and(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::And, Box::new(rhs.into()))
    }

    pub fn add(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Add, Box::new(rhs.into()))
    }

    pub fn mul(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Mul, Box::new(rhs.into()))
    }

    pub fn mud(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Mod, Box::new(rhs.into()))
    }

    pub fn eq(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Eq, Box::new(rhs.into()))
    }

    pub fn lt(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Lt, Box::new(rhs.into()))
    }

    pub fn gt(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Gt, Box::new(rhs.into()))
    }

    pub fn le(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Le, Box::new(rhs.into()))
    }

    pub fn ge(self, rhs: impl Into<Expr>) -> Self {
        Expr::Binary(Box::new(self), BinaryOp::Ge, Box::new(rhs.into()))
    }

    pub fn len(self) -> Self {
        Expr::Unary(UnaryOp::Len, Box::new(self))
    }

    pub fn bit(self) -> Self {
        Expr::Bit(Box::new(self))
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Boolean(b) => write!(f, "{b}"),
            Expr::Number(n) => write!(f, "{n}"),
            Expr::String(s) => write!(f, "\"{}\"", s.escape_default()),

            Expr::Table(fields) => {
                let fields = fields
                    .iter()
                    .map(|(i, v)| format!("[{i}] = {v}"))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "{{ {fields} }}")
            }

            Expr::Array(expr) if matches!(**expr, Expr::Number(0.0)) => write!(f, "{{}}"),
            Expr::Array(expr) => write!(f, "table.create({expr})"),

            Expr::Var(v) => write!(f, "{v}"),

            Expr::Index(expr, index) => write!(f, "{expr}[{index}]"),

            Expr::Binary(lhs, op, rhs) => write!(f, "({lhs} {op} {rhs})"),
            Expr::Unary(op, expr) => write!(f, "{op}{expr}"),

            Expr::Vector(x, y, z) => write!(f, "vector.create({x}, {y}, {z})"),
            Expr::Type(expr) => write!(f, "type({expr})"),
            Expr::Utf8(expr) => write!(f, "utf8.len({expr})"),
            Expr::Bit(expr) => write!(f, "bit[{expr}]"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum BinaryOp {
    And,

    Add,
    Mul,
    Mod,

    Eq,
    Lt,
    Gt,
    Le,
    Ge,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::And => write!(f, "and"),
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Ge => write!(f, ">="),
        }
    }
}

#[derive(Clone, Copy)]
pub enum UnaryOp {
    Len,
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOp::Len => write!(f, "#"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Var(pub u16);

impl Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}
