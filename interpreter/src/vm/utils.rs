use crate::compiler::byte_code::Address;
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

pub type Pointer = u64;
pub type ChannelId = u64;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Int(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Value, Value>),
    List(Vec<Value>),
    Closure { captured: Vec<Value>, body: Address },
    ChannelSendEnd(ChannelId),
    ChannelReceiveEnd(ChannelId),
}
impl Value {
    pub fn unit() -> Self {
        Self::Symbol("".into())
    }
    pub fn int(self) -> Option<i64> {
        match self {
            Self::Int(int) => Some(int),
            _ => None,
        }
    }
    pub fn string(self) -> Option<String> {
        match self {
            Self::String(string) => Some(string),
            _ => None,
        }
    }
    pub fn symbol(self) -> Option<String> {
        match self {
            Self::Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }
    pub fn map(self) -> Option<HashMap<Value, Value>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }
    pub fn list(self) -> Option<Vec<Value>> {
        match self {
            Self::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn closure(self) -> Option<(Vec<Value>, Address)> {
        match self {
            Self::Closure { captured, body } => Some((captured, body)),
            _ => None,
        }
    }
    pub fn channel_send_end(self) -> Option<u64> {
        match self {
            Self::ChannelSendEnd(channel_send_end) => Some(channel_send_end),
            _ => None,
        }
    }
    pub fn channel_receive_end(self) -> Option<u64> {
        match self {
            Self::ChannelReceiveEnd(channel_receive_end) => Some(channel_receive_end),
            _ => None,
        }
    }
}
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(int) => write!(f, "{}", int),
            Value::String(string) => write!(f, "{:?}", string),
            Value::Symbol(symbol) => write!(f, ":{}", symbol),
            Value::Map(map) => write!(
                f,
                "{{{}}}",
                itertools::join(
                    map.iter()
                        .map(|(key, value)| format!("{}, {}", &key, &value,)),
                    ", "
                )
            ),
            Value::List(list) => write!(
                f,
                "({})",
                itertools::join(list.iter().map(|item| item.to_string()), ", ")
            ),
            Value::Closure {
                captured: _,
                body: _,
            } => write!(f, "<closure>"),
            Value::ChannelSendEnd(channel_id) => write!(f, "<send end for channel {}>", channel_id),
            Value::ChannelReceiveEnd(channel_id) => {
                write!(f, "<receive end for channel {}>", channel_id)
            }
        }
    }
}
