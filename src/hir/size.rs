use std::ops::{Add, Mul};

use crate::hir;

#[derive(Default, Clone, Copy)]
pub struct Size {
    pub min: u32,
    pub max: Option<u32>,
}

impl Add<u32> for Size {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self {
            min: self.min + rhs,
            max: self.max.map(|m| m + rhs),
        }
    }
}

impl Add<Size> for Size {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            min: self.min + rhs.min,
            max: match (self.max, rhs.max) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            },
        }
    }
}

impl Mul<u32> for Size {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        Self {
            min: self.min * rhs,
            max: self.max.map(|m| m * rhs),
        }
    }
}

impl Mul<hir::Length> for Size {
    type Output = Self;

    fn mul(self, rhs: hir::Length) -> Self::Output {
        let min = rhs.min;
        let max = rhs.max;

        Self {
            min: self.min * min,
            max: match (self.max, max) {
                (Some(a), Some(b)) => Some(a * b),
                _ => None,
            },
        }
    }
}

impl From<u32> for Size {
    fn from(value: u32) -> Self {
        Self {
            min: value,
            max: Some(value),
        }
    }
}

impl Size {
    pub fn is_exact(&self) -> Option<u32> {
        if Some(self.min) == self.max {
            self.max
        } else {
            None
        }
    }
}

impl hir::Type {
    pub fn size(&self) -> Size {
        match self {
            hir::Type::Boolean(ty) => ty.size(),
            hir::Type::Number(ty) => ty.size(),
            hir::Type::Vector(ty) => ty.size(),
            hir::Type::BinaryString(ty) => ty.size(),
            hir::Type::Utf8String(ty) => ty.size(),
            hir::Type::Array(ty) => ty.size(),
            hir::Type::Set(ty) => ty.size(),
            hir::Type::Map(ty) => ty.size(),
            hir::Type::Enum(ty) => ty.size(),
            hir::Type::Struct(ty) => ty.size(),
        }
    }
}

impl hir::BooleanType {
    pub fn size(&self) -> Size {
        Size::from(1)
    }
}

impl hir::NumberType {
    pub fn size(&self) -> Size {
        Size {
            min: self.kind.size(),
            max: Some(self.kind.size()),
        }
    }
}

impl hir::VectorType {
    pub fn size(&self) -> Size {
        self.x.size() + self.y.size() + self.z.as_ref().map_or(Size::default(), |z| z.size())
    }
}

impl hir::BinaryStringType {
    pub fn size(&self) -> Size {
        Size::from(1) * self.len + self.len.kind().size()
    }
}

impl hir::Utf8StringType {
    pub fn size(&self) -> Size {
        Size::from(1) * self.len + self.len.kind().size()
    }
}

impl hir::ArrayType {
    pub fn size(&self) -> Size {
        self.item.size() * self.len + self.len.kind().size()
    }
}

impl hir::SetType {
    pub fn size(&self) -> Size {
        self.item.size() * self.len + self.len.kind().size()
    }
}

impl hir::MapType {
    pub fn size(&self) -> Size {
        self.index.size() + self.value.size() * self.len + self.len.kind().size()
    }
}

impl hir::EnumType {
    pub fn size(&self) -> Size {
        self.number.size()
    }
}

impl hir::StructType {
    pub fn size(&self) -> Size {
        self.fields
            .iter()
            .map(|(_, field_type)| field_type.size())
            .fold(Size::default(), |acc, size| acc + size)
    }
}
