#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct NumberType {
    pub kind: NumberKind,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct VectorType {
    pub x: NumberType,
    pub y: NumberType,
    pub z: Option<NumberType>,
}

#[derive(Debug, Clone)]
pub struct BinaryStringType {
    pub len: Range,
}

#[derive(Debug, Clone)]
pub struct Utf8StringType {
    pub len: Range,
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub len: Range,
    pub item: Box<Type>,
}

#[derive(Debug, Clone)]
pub struct SetType {
    pub len: Range,
    pub item: Box<Type>,
}

#[derive(Debug, Clone)]
pub struct MapType {
    pub len: Range,
    pub index: Box<Type>,
    pub value: Box<Type>,
}

#[derive(Debug, Clone)]
pub struct StructType {
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl Range {
    pub fn exact(&self) -> Option<f64> {
        if self.min == self.max { self.min } else { None }
    }

    pub fn len_kind(&self) -> NumberKind {
        let max = self.max.unwrap_or(f64::MAX);

        if max <= u8::MAX as f64 {
            NumberKind::U8
        } else if max <= u16::MAX as f64 {
            NumberKind::U16
        } else {
            NumberKind::U32
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NumberKind {
    U8,
    U16,
    U32,

    I8,
    I16,
    I32,

    F32,
    F64,

    NaNF32,
    NaNF64,
}

impl NumberKind {
    pub fn size(&self) -> u32 {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 => 4,
            Self::F32 | Self::NaNF32 => 4,
            Self::F64 | Self::NaNF64 => 8,
        }
    }
}
