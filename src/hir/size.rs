use std::ops::{Add, Mul};

use crate::hir;

#[derive(Clone, Copy, PartialEq)]
pub struct Size {
    pub min: u32,
    pub max: Option<u32>,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            min: 0,
            max: Some(0),
        }
    }
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

impl PartialOrd for Size {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.max, other.max) {
            (Some(a), Some(b)) => Some(self.min.cmp(&other.min).then(a.cmp(&b))),
            (Some(_), None) => Some(std::cmp::Ordering::Less),
            (None, Some(_)) => Some(std::cmp::Ordering::Greater),
            (None, None) => Some(std::cmp::Ordering::Equal),
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

pub trait SizeOf {
    fn size_of(&self) -> Size;
}

impl<T: SizeOf> SizeOf for Vec<T> {
    fn size_of(&self) -> Size {
        self.iter()
            .map(|item| item.size_of())
            .fold(Size::default(), |acc, size| acc + size)
    }
}

impl SizeOf for hir::Type {
    fn size_of(&self) -> Size {
        match self {
            hir::Type::Boolean(ty) => ty.size_of(),
            hir::Type::Number(ty) => ty.size_of(),
            hir::Type::Vector(ty) => ty.size_of(),
            hir::Type::BinaryString(ty) => ty.size_of(),
            hir::Type::Utf8String(ty) => ty.size_of(),
            hir::Type::Array(ty) => ty.size_of(),
            hir::Type::Set(ty) => ty.size_of(),
            hir::Type::Map(ty) => ty.size_of(),
            hir::Type::Enum(ty) => ty.size_of(),
            hir::Type::Struct(ty) => ty.size_of(),
        }
    }
}

impl SizeOf for hir::BooleanType {
    fn size_of(&self) -> Size {
        Size::from(1)
    }
}

impl SizeOf for hir::NumberType {
    fn size_of(&self) -> Size {
        Size {
            min: self.kind.size(),
            max: Some(self.kind.size()),
        }
    }
}

impl SizeOf for hir::VectorType {
    fn size_of(&self) -> Size {
        self.x.size_of()
            + self.y.size_of()
            + self.z.as_ref().map_or(Size::default(), |z| z.size_of())
    }
}

impl SizeOf for hir::BinaryStringType {
    fn size_of(&self) -> Size {
        Size::from(1) * self.len + self.len.kind().size()
    }
}

impl SizeOf for hir::Utf8StringType {
    fn size_of(&self) -> Size {
        Size::from(1) * self.len + self.len.kind().size()
    }
}

impl SizeOf for hir::ArrayType {
    fn size_of(&self) -> Size {
        self.item.size_of() * self.len + self.len.kind().size()
    }
}

impl SizeOf for hir::SetType {
    fn size_of(&self) -> Size {
        self.item.size_of() * self.len + self.len.kind().size()
    }
}

impl SizeOf for hir::MapType {
    fn size_of(&self) -> Size {
        self.index.size_of() + self.value.size_of() * self.len + self.len.kind().size()
    }
}

impl SizeOf for hir::EnumType {
    fn size_of(&self) -> Size {
        self.number.size_of()
    }
}

impl SizeOf for hir::StructType {
    fn size_of(&self) -> Size {
        self.fields
            .iter()
            .map(|(_, field_type)| field_type.size_of())
            .fold(Size::default(), |acc, size| acc + size)
    }
}
