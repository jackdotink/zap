use uuid::Uuid;

use crate::shared::{NetworkSide, NumberKind, Range};

mod build;
mod size;

#[derive(Clone)]
pub enum Item {
    Event(Event),
    Table(Vec<(String, Item)>),
}

#[derive(Clone)]
pub struct Event {
    pub uuid: Uuid,
    pub from: NetworkSide,
    pub data: Vec<Type>,
}

#[derive(Clone)]
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
pub struct StructType {
    pub fields: Vec<(String, Type)>,
}

#[derive(Clone, Copy)]
pub struct Length {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

impl Length {
    pub fn exact(&self) -> Option<u32> {
        if self.min == self.max { self.min } else { None }
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

    pub fn number_type(&self) -> NumberType {
        NumberType {
            kind: self.kind(),
            range: Range {
                min: self.min.map(|min| min as f64),
                max: self.max.map(|max| max as f64),
            },
        }
    }
}

impl From<Length> for Range {
    fn from(value: Length) -> Self {
        Range {
            min: value.min.map(|min| min as f64),
            max: value.max.map(|max| max as f64),
        }
    }
}
