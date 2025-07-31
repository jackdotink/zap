use std::rc::Rc;

#[derive(Default, Clone, Copy)]
pub enum ApiCheck {
    None,
    #[default]
    Some,
    Full,
}

#[derive(Default, Clone, Copy)]
pub enum Casing {
    #[default]
    Lower,
    Snake,
    Camel,
    Pascal,
}

impl Casing {
    pub fn fmt(&self, words: &'static str) -> String {
        fn capitalize(word: &str) -> String {
            let mut chars = word.chars();
            format!(
                "{}{}",
                chars.next().unwrap().to_ascii_uppercase(),
                chars.as_str()
            )
        }

        let words = words.split(' ').collect::<Vec<_>>();
        match self {
            Self::Lower => words.concat(),
            Self::Snake => words.join("_"),
            Self::Camel => words
                .iter()
                .enumerate()
                .map(|(i, word)| {
                    if i == 0 {
                        word.to_string()
                    } else {
                        capitalize(word)
                    }
                })
                .collect(),
            Self::Pascal => words.into_iter().map(capitalize).collect(),
        }
    }
}

#[derive(lu::Userdata, Default, Clone)]
pub struct Options {
    parent: Option<Rc<Options>>,

    apicheck: Option<ApiCheck>,
    casing: Option<Casing>,
}

impl Options {
    pub fn with_parent(self, parent: Rc<Options>) -> Self {
        Self {
            parent: Some(parent),
            apicheck: self.apicheck,
            casing: self.casing,
        }
    }

    pub fn with_apicheck(self, apicheck: ApiCheck) -> Self {
        Self {
            parent: self.parent,
            apicheck: Some(apicheck),
            casing: self.casing,
        }
    }

    pub fn with_casing(self, casing: Casing) -> Self {
        Self {
            parent: self.parent,
            apicheck: self.apicheck,
            casing: Some(casing),
        }
    }

    pub fn apicheck(&self) -> ApiCheck {
        self.apicheck.unwrap_or_else(|| {
            self.parent
                .as_ref()
                .map_or(ApiCheck::default(), |parent| parent.apicheck())
        })
    }

    pub fn casing(&self) -> Casing {
        self.casing.unwrap_or_else(|| {
            self.parent
                .as_ref()
                .map_or(Casing::default(), |parent| parent.casing())
        })
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

    pub fn kind(&self) -> NumberKind {
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
