#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PrimitiveKind {
    Add,
    GetItem,
    GetAmbient,
    Panic,
    CreateChannel,
    Send,
    Receive,
}

impl PrimitiveKind {
    pub fn parse(symbol: &str) -> Option<PrimitiveKind> {
        Some(match symbol {
            "add" => Self::Add,
            "get-item" => Self::GetItem,
            "get-ambient" => Self::GetAmbient,
            "panic" => Self::Panic,
            "channel" => Self::CreateChannel,
            "send" => Self::Send,
            "receive" => Self::Receive,
            _ => return None,
        })
    }

    pub const fn all() -> [PrimitiveKind; 7] {
        [
            PrimitiveKind::Add,
            PrimitiveKind::GetItem,
            PrimitiveKind::GetAmbient,
            PrimitiveKind::Panic,
            PrimitiveKind::CreateChannel,
            PrimitiveKind::Send,
            PrimitiveKind::Receive,
        ]
    }

    pub fn is_pure(&self) -> bool {
        match self {
            Self::Add => true,
            Self::GetItem => true,
            Self::GetAmbient => false,
            Self::Panic => false,
            Self::CreateChannel => false,
            Self::Send => false,
            Self::Receive => false,
        }
    }
}
