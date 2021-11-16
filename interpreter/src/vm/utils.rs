use crate::compiler::byte_code::{Address, ByteCode};
use std::collections::HashMap;

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
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}
