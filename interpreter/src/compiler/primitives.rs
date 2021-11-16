#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PrimitiveKind {
    Add,
    GetAmbient,
    Send,
    Print,
}

impl PrimitiveKind {
    pub fn parse(symbol: &str) -> Option<PrimitiveKind> {
        Some(match symbol {
            "add" => Self::Add,
            "get-ambient" => Self::GetAmbient,
            "send" => Self::Send,
            "print" => Self::Print,
            _ => return None,
        })
    }

    pub fn is_pure(&self) -> bool {
        match self {
            Self::Add => true,
            Self::GetAmbient => false,
            Self::Send => false,
            Self::Print => false,
        }
    }
}
