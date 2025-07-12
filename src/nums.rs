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

    pub fn min(&self) -> f64 {
        match self {
            Self::U8 => 0.0,
            Self::U16 => 0.0,
            Self::U32 => 0.0,
            Self::I8 => i8::MIN as f64,
            Self::I16 => i16::MIN as f64,
            Self::I32 => i32::MIN as f64,
            Self::F32 | Self::NaNF32 => f32::MIN as f64,
            Self::F64 | Self::NaNF64 => f64::MIN as f64,
        }
    }

    pub fn max(&self) -> f64 {
        match self {
            Self::U8 => u8::MAX as f64,
            Self::U16 => u16::MAX as f64,
            Self::U32 => u32::MAX as f64,
            Self::I8 => i8::MAX as f64,
            Self::I16 => i16::MAX as f64,
            Self::I32 => i32::MAX as f64,
            Self::F32 | Self::NaNF32 => f32::MAX as f64,
            Self::F64 | Self::NaNF64 => f64::MAX as f64,
        }
    }
}
