use crate::{
    options::Options,
    shared::{NetworkSide, NumberKind, Range, Remote},
};

mod buckets;
mod build;
mod display;
mod size;

pub use buckets::Buckets;
pub use build::build;

pub struct Hir {
    pub table: Table,
}

#[derive(Clone)]
pub struct Table {
    pub items: Vec<(String, Item)>,
}

#[derive(Clone)]
pub enum Item {
    Table(Table),
    Event(Event),
}

#[derive(Clone)]
pub struct Event {
    pub opts: Options,
    pub thru: Remote,
    pub from: NetworkSide,
    pub data: Vec<Type>,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct NumberType {
    pub kind: NumberKind,
    pub range: Range,
}

#[derive(Clone)]
pub struct VectorType {
    pub x: NumberType,
    pub y: NumberType,
    pub z: Option<NumberType>,
}

#[derive(Clone)]
pub struct BinaryStringType {
    pub len: Length,
}

#[derive(Clone)]
pub struct Utf8StringType {
    pub len: Length,
}

#[derive(Clone)]
pub struct ArrayType {
    pub len: Length,
    pub item: Box<Type>,
}

#[derive(Clone)]
pub struct SetType {
    pub len: Length,
    pub item: Box<Type>,
}

#[derive(Clone)]
pub struct MapType {
    pub len: Length,
    pub index: Box<Type>,
    pub value: Box<Type>,
}

#[derive(Clone)]
pub struct EnumType {
    pub variants: Vec<String>,
    pub number: NumberType,
}

#[derive(Clone)]
pub struct StructType {
    pub fields: Vec<(String, Type)>,
}

#[derive(Clone, Copy)]
pub struct Length {
    pub min: u32,
    pub max: Option<u32>,
}

impl Length {
    pub fn exact(&self) -> Option<u32> {
        if let Some(max) = self.max
            && self.min == max
        {
            Some(max)
        } else {
            None
        }
    }

    pub fn kind(&self) -> NumberKind {
        let max = self.max.unwrap_or(u32::MAX);

        if max <= u8::MAX as u32 {
            NumberKind::U8
        } else if max <= u16::MAX as u32 {
            NumberKind::U16
        } else {
            NumberKind::U32
        }
    }
}

impl From<Length> for Range {
    fn from(value: Length) -> Self {
        Range {
            min: Some(value.min as f64),
            max: value.max.map(|max| max as f64),
        }
    }
}
