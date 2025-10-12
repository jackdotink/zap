use crate::{
    api,
    hir::{
        ArrayType, BinaryStringType, BooleanType, EnumType, Event, Hir, Item, Length, MapType,
        NumberType, SetType, StructType, Table, Type, Utf8StringType, VectorType,
    },
    shared::Range,
};

pub fn build(table: api::Table) -> Hir {
    Hir {
        table: table.into(),
    }
}

impl From<api::Table> for Table {
    fn from(value: api::Table) -> Self {
        let opts = value.opts.resolved();
        let mut items = Vec::new();

        for (name, item) in value.items {
            let item = match item {
                api::Item::Table(table) => Item::Table(table.into()),
                api::Item::Event(event) => Item::Event(Event {
                    opts: opts.clone(),
                    thru: event.thru,
                    from: event.from,
                    data: event.data.into_iter().map(Type::from).collect(),
                }),
            };

            items.push((name, item))
        }

        Table { items }
    }
}

impl From<api::Type> for Type {
    fn from(value: api::Type) -> Self {
        match value {
            api::Type::Boolean(ty) => Type::Boolean(ty.into()),
            api::Type::Number(ty) => Type::Number(ty.into()),
            api::Type::Vector(ty) => Type::Vector(ty.into()),
            api::Type::BinaryString(ty) => Type::BinaryString(ty.into()),
            api::Type::Utf8String(ty) => Type::Utf8String(ty.into()),
            api::Type::Array(ty) => Type::Array(ty.into()),
            api::Type::Set(ty) => Type::Set(ty.into()),
            api::Type::Map(ty) => Type::Map(ty.into()),
            api::Type::Enum(ty) => Type::Enum(ty.into()),
            api::Type::Struct(ty) => Type::Struct(ty.into()),
        }
    }
}

impl From<api::BooleanType> for BooleanType {
    fn from(_: api::BooleanType) -> Self {
        BooleanType
    }
}

impl From<api::NumberType> for NumberType {
    fn from(value: api::NumberType) -> Self {
        let kind = value.kind;
        let range = value.range;

        NumberType { kind, range }
    }
}

impl From<api::VectorType> for VectorType {
    fn from(value: api::VectorType) -> Self {
        let x = NumberType::from(value.x);
        let y = NumberType::from(value.y);
        let z = value.z.map(NumberType::from);

        VectorType { x, y, z }
    }
}

impl From<api::BinaryStringType> for BinaryStringType {
    fn from(value: api::BinaryStringType) -> Self {
        let len = Length::from(value.len);

        BinaryStringType { len }
    }
}

impl From<api::Utf8StringType> for Utf8StringType {
    fn from(value: api::Utf8StringType) -> Self {
        let len = Length::from(value.len);

        Utf8StringType { len }
    }
}

impl From<api::ArrayType> for ArrayType {
    fn from(value: api::ArrayType) -> Self {
        let len = Length::from(value.len);
        let item = Box::new(Type::from(*value.item));

        ArrayType { len, item }
    }
}

impl From<api::SetType> for SetType {
    fn from(value: api::SetType) -> Self {
        let len = Length::from(value.len);
        let item = Box::new(Type::from(*value.item));

        SetType { len, item }
    }
}

impl From<api::MapType> for MapType {
    fn from(value: api::MapType) -> Self {
        let len = Length::from(value.len);
        let index = Box::new(Type::from(*value.index));
        let value = Box::new(Type::from(*value.value));

        MapType { len, index, value }
    }
}

impl From<api::EnumType> for EnumType {
    fn from(value: api::EnumType) -> Self {
        let variants = value.variants;
        let range = Range {
            min: Some(0f64),
            max: Some(variants.len() as f64 - 1f64),
        };
        let number = NumberType {
            kind: range.kind(),
            range,
        };

        EnumType { variants, number }
    }
}

impl From<api::StructType> for StructType {
    fn from(value: api::StructType) -> Self {
        let fields = value
            .fields
            .into_iter()
            .map(|(name, ty)| (name, Type::from(ty)))
            .collect();

        StructType { fields }
    }
}

impl From<Range> for Length {
    fn from(value: Range) -> Self {
        let min = value.min.map(|n| n as u32).unwrap_or(0);
        let max = value.max.map(|n| n as u32);

        Length { min, max }
    }
}
