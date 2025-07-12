use crate::nums::NumberKind;

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
