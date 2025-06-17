use crate::types::{
    ArrayType, BinaryStringType, MapType, NumberKind, NumberType, Range, StructType, Type,
    VectorType,
};

mod ir;
mod luau;
mod types;

fn main() {
    let ty = Type::Struct(StructType {
        fields: vec![
            (
                "foo".to_string(),
                Type::Number(NumberType {
                    kind: NumberKind::I16,
                    range: Range {
                        min: None,
                        max: None,
                    },
                }),
            ),
            (
                "bar".to_string(),
                Type::Array(ArrayType {
                    len: Range {
                        min: Some(1.0),
                        max: Some(150.0),
                    },
                    item: Box::new(Type::Number(NumberType {
                        kind: NumberKind::F32,
                        range: Range {
                            min: None,
                            max: None,
                        },
                    })),
                }),
            ),
            (
                "baz".to_string(),
                Type::Map(MapType {
                    len: Range {
                        min: None,
                        max: None,
                    },
                    index: Box::new(Type::BinaryString(BinaryStringType {
                        len: Range {
                            min: None,
                            max: None,
                        },
                    })),
                    value: Box::new(Type::Vector(VectorType {
                        x: NumberType {
                            kind: NumberKind::U8,
                            range: Range {
                                min: None,
                                max: None,
                            },
                        },
                        y: NumberType {
                            kind: NumberKind::U16,
                            range: Range {
                                min: None,
                                max: None,
                            },
                        },
                        z: None,
                    })),
                }),
            ),
        ],
    });

    let block = ir::build(ty);
    println!("{}", block);
}
