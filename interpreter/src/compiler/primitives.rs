#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PrimitiveKind {
    Add,
    Print,
}

impl PrimitiveKind {
    pub fn parse(symbol: &str) -> Option<PrimitiveKind> {
        Some(match symbol {
            "add" => Self::Add,
            "print" => Self::Print,
            _ => return None,
        })
    }

    pub fn is_pure(&self) -> bool {
        match self {
            Self::Add => true,
            Self::Print => false,
        }
    }
}
