#[derive(Clone, Copy)]
pub enum ApiCheck {
    None,
    Some,
    Full,
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

#[derive(Debug, Clone, Copy)]
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

#[derive(Clone, Copy)]
pub enum NetworkSide {
    Server,
    Client,
}
