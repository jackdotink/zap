use std::fmt::Display;

use crate::hir::{
    ArrayType, BinaryStringType, BooleanType, EnumType, MapType, NumberType, SetType, StructType,
    Type, Utf8StringType, VectorType,
};

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean(ty) => write!(f, "{ty}"),
            Self::Number(ty) => write!(f, "{ty}"),
            Self::Vector(ty) => write!(f, "{ty}"),
            Self::BinaryString(ty) => write!(f, "{ty}"),
            Self::Utf8String(ty) => write!(f, "{ty}"),
            Self::Array(ty) => write!(f, "{ty}"),
            Self::Set(ty) => write!(f, "{ty}"),
            Self::Map(ty) => write!(f, "{ty}"),
            Self::Enum(ty) => write!(f, "{ty}"),
            Self::Struct(ty) => write!(f, "{ty}"),
        }
    }
}

impl Display for BooleanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "boolean")
    }
}

impl Display for NumberType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "number")
    }
}

impl Display for VectorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vector")
    }
}

impl Display for BinaryStringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "string")
    }
}

impl Display for Utf8StringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "string")
    }
}

impl Display for ArrayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {} }}", self.item)
    }
}

impl Display for SetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ [{}]: boolean }}", self.item)
    }
}

impl Display for MapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ [{}]: {} }}", self.index, self.value)
    }
}

impl Display for EnumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        for variant in &self.variants {
            write!(f, "| {variant}")?;
        }
        write!(f, ")")
    }
}

impl Display for StructType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (name, ty) in &self.fields {
            write!(f, "[\"{name}\"]: {ty},")?;
        }
        write!(f, "}}")
    }
}
