use std::fmt::Display;

use lu::Userdata;
use uuid::Uuid;

use crate::{nums::NumberKind, range::Range};

#[derive(Debug, Clone)]
pub enum Item {
    Event(Event),
    Table(Vec<(String, Item)>),
}

#[derive(Debug, Clone, Userdata)]
pub struct Event {
    pub name: Uuid,
    pub from: EventFrom,
    pub data: Vec<Type>,
    pub reliable: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum EventFrom {
    Server,
    Client,
}

#[derive(Debug, Clone, Userdata)]
pub enum Type {
    Number(NumberType),
    Vector(VectorType),
    BinaryString(BinaryStringType),
    Utf8String(Utf8StringType),
    Array(ArrayType),
    Set(SetType),
    Map(MapType),
    Struct(StructType),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Number(ty) => write!(f, "{ty}"),
            Type::Vector(ty) => write!(f, "{ty}"),
            Type::BinaryString(ty) => write!(f, "{ty}"),
            Type::Utf8String(ty) => write!(f, "{ty}"),
            Type::Array(ty) => write!(f, "{ty}"),
            Type::Set(ty) => write!(f, "{ty}"),
            Type::Map(ty) => write!(f, "{ty}"),
            Type::Struct(ty) => write!(f, "{ty}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NumberType {
    pub kind: NumberKind,
    pub range: Range,
}

impl Display for NumberType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "number")
    }
}

#[derive(Debug, Clone)]
pub struct VectorType {
    pub x: NumberType,
    pub y: NumberType,
    pub z: Option<NumberType>,
}

impl Display for VectorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vector")
    }
}

#[derive(Debug, Clone)]
pub struct BinaryStringType {
    pub len: Range,
}

impl Display for BinaryStringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "string")
    }
}

#[derive(Debug, Clone)]
pub struct Utf8StringType {
    pub len: Range,
}

impl Display for Utf8StringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "string")
    }
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub len: Range,
    pub item: Box<Type>,
}

impl Display for ArrayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {} }}", self.item)
    }
}

#[derive(Debug, Clone)]
pub struct SetType {
    pub len: Range,
    pub item: Box<Type>,
}

impl Display for SetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ [{}]: unknown }}", self.item)
    }
}

#[derive(Debug, Clone)]
pub struct MapType {
    pub len: Range,
    pub index: Box<Type>,
    pub value: Box<Type>,
}

impl Display for MapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ [{}]: {} }}", self.index, self.value)
    }
}

#[derive(Debug, Clone)]
pub struct StructType {
    pub fields: Vec<(String, Type)>,
}

impl Display for StructType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ ")?;
        for (name, ty) in &self.fields {
            write!(f, "[\"{}\"]: {ty}, ", name.escape_default())?;
        }
        write!(f, "}}")
    }
}
