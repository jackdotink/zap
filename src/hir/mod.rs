use crate::{nums::NumberKind, range::Range};

mod size;

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

impl NumberType {
    pub fn size(&self) -> u32 {
        self.kind.size()
    }
}

#[derive(Clone)]
pub struct VectorType {
    pub x: NumberType,
    pub y: NumberType,
    pub z: Option<NumberType>,
}

#[derive(Clone)]
pub struct BinaryStringType {
    pub len: NumberType,
}

#[derive(Clone)]
pub struct Utf8StringType {
    pub len: NumberType,
}

#[derive(Clone)]
pub struct ArrayType {
    pub len: NumberType,
    pub item: Box<Type>,
}

#[derive(Clone)]
pub struct SetType {
    pub len: NumberType,
    pub item: Box<Type>,
}

#[derive(Clone)]
pub struct MapType {
    pub len: NumberType,
    pub index: Box<Type>,
    pub value: Box<Type>,
}

#[derive(Clone)]
pub struct StructType {
    pub fields: Vec<(String, Type)>,
}
