#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PrimitiveKind {
    Add,
    GetAmbient,
    Send,
    Receive,
}

impl PrimitiveKind {
    pub fn parse(symbol: &str) -> Option<PrimitiveKind> {
        Some(match symbol {
            "add" => Self::Add,
            "get-ambient" => Self::GetAmbient,
            "send" => Self::Send,
            "receive" => Self::Receive,
            _ => return None,
        })
    }

    pub fn is_pure(&self) -> bool {
        match self {
            Self::Add => true,
            Self::GetAmbient => false,
            Self::Send => false,
            Self::Receive => false,
        }
    }
}
