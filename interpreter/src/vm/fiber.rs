use super::utils::*;
use crate::compiler::byte_code::{Address, ByteCode, Instruction, StackOffset};
use crate::compiler::PrimitiveKind;
use crate::utils::*;
use itertools::Itertools;
use log::debug;
use std::collections::HashMap;

/// A fiber can execute some byte code. It's "single-threaded", a pure
/// mathematical machine and only communicates with the outside world through
/// channels, which can be provided during instantiation as ambients.
#[derive(Debug)]
pub struct Fiber {
    byte_code: ByteCode,
    ambients: HashMap<String, Value>,
    status: FiberStatus,
    ip: Pointer, // instruction pointer
    stack: Vec<StackEntry>,
    heap: HashMap<Pointer, Object>, // TODO: use the actual real pointers objects
    next_heap_address: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FiberStatus {
    Running,
    Done(Value),
    Panicked(Value),
    CreatingChannel(u64),
    Sending(ChannelId, Value),
    Receiving(ChannelId),
}

// TODO: Unify both cases and inline some values. The size of the stack is very
// performance-critical, so explicitly controlling its memory layout would be
// nice.
#[derive(Debug, Clone)]
enum StackEntry {
    AddressInByteCode(Address),
    AddressInHeap(Pointer),
}

#[derive(Debug, Clone)] // TODO: rm Clone
pub struct Object {
    reference_count: u64,
    data: ObjectData,
}
#[derive(Clone, Debug)] // TODO: rm Clone
pub enum ObjectData {
    Int(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Pointer, Pointer>),
    List(Vec<Pointer>),
    Closure {
        captured: Vec<Pointer>,
        body: Address,
    },
    ChannelSendEnd(ChannelId),
    ChannelReceiveEnd(ChannelId),
}

impl Fiber {
    pub fn new(byte_code: ByteCode, ambients: HashMap<String, Value>, dot: Value) -> Self {
        let mut fiber = Self {
            byte_code,
            ambients,
            status: FiberStatus::Running,
            ip: 0,
            stack: vec![],
            heap: HashMap::new(),
            next_heap_address: 123450000,
        };
        let address = fiber.import(dot);
        fiber.stack.push(StackEntry::AddressInHeap(address));
        fiber
    }

    pub fn status(&self) -> FiberStatus {
        self.status.clone()
    }

    fn get_from_stack(&self, offset: StackOffset) -> StackEntry {
        debug!("Getting offset {} from stack {:?}.", offset, self.stack);
        self.stack[self.stack.len() - (offset as usize) - 1].clone()
    }
    fn get_from_heap(&mut self, address: Pointer) -> &mut Object {
        self.heap.get_mut(&address).unwrap()
    }

    fn create_object(&mut self, object: ObjectData) -> Pointer {
        let address = self.next_heap_address;
        self.heap.insert(
            address,
            Object {
                reference_count: 1,
                data: object,
            },
        );
        self.next_heap_address += 1;
        address
    }
    fn free_object(&mut self, address: Pointer) {
        self.heap.remove(&address);
    }

    fn dup(&mut self, address: Pointer) {
        let object = self.get_from_heap(address);
        object.reference_count += 1;
    }
    fn drop(&mut self, address: Pointer) {
        let object = self.get_from_heap(address);
        object.reference_count -= 1;
        let object = object.clone();
        if object.reference_count == 0 {
            match &object.data {
                ObjectData::Int(_) | ObjectData::String(_) | ObjectData::Symbol(_) => {}
                ObjectData::Map(map) => {
                    for (key, value) in map {
                        self.drop(*key);
                        self.drop(*value);
                    }
                }
                ObjectData::List(list) => {
                    for item in list {
                        self.drop(*item);
                    }
                }
                ObjectData::Closure { captured, .. } => {
                    for object in captured {
                        self.drop(*object);
                    }
                }
                ObjectData::ChannelSendEnd(_) | ObjectData::ChannelReceiveEnd(_) => {}
            }
            self.free_object(address);
        }
    }

    fn import(&mut self, value: Value) -> Pointer {
        let data = match value {
            Value::Int(int) => ObjectData::Int(int),
            Value::String(string) => ObjectData::String(string),
            Value::Symbol(symbol) => ObjectData::Symbol(symbol),
            Value::Map(map) => {
                let mut map_data = HashMap::new();
                for (key, value) in map {
                    map_data.insert(self.import(key), self.import(value));
                }
                ObjectData::Map(map_data)
            }
            Value::List(list) => {
                let mut list_data = vec![];
                for item in list {
                    list_data.push(self.import(item));
                }
                ObjectData::List(list_data)
            }
            Value::Closure { captured, body } => {
                let mut captured_objects = vec![];
                for value in captured {
                    captured_objects.push(self.import(value));
                }
                ObjectData::Closure {
                    captured: captured_objects,
                    body,
                }
            }
            Value::ChannelSendEnd(channel_id) => ObjectData::ChannelSendEnd(channel_id),
            Value::ChannelReceiveEnd(channel_id) => ObjectData::ChannelReceiveEnd(channel_id),
        };
        self.create_object(data)
    }
    fn export(&mut self, address: Pointer) -> Value {
        let value = self.export_helper(address);
        self.drop(address);
        value
    }
    fn export_helper(&mut self, address: Pointer) -> Value {
        match self.get_from_heap(address).data.clone() {
            ObjectData::Int(int) => Value::Int(int),
            ObjectData::String(string) => Value::String(string),
            ObjectData::Symbol(symbol) => Value::Symbol(symbol),
            ObjectData::Map(map) => {
                let mut map_value = HashMap::new();
                for (key, value) in map {
                    map_value.insert(self.export_helper(key), self.export_helper(value));
                }
                Value::Map(map_value)
            }
            ObjectData::List(list) => {
                let mut list_value = vec![];
                for item in list {
                    list_value.push(self.export_helper(item));
                }
                Value::List(list_value)
            }
            ObjectData::Closure { captured, body } => {
                let mut captured_values = vec![];
                for object in captured {
                    captured_values.push(self.export_helper(object));
                }
                Value::Closure {
                    captured: captured_values,
                    body,
                }
            }
            ObjectData::ChannelSendEnd(channel_id) => Value::ChannelSendEnd(channel_id),
            ObjectData::ChannelReceiveEnd(channel_id) => Value::ChannelReceiveEnd(channel_id),
        }
    }

    pub fn run(&mut self, mut num_instructions: u16) {
        assert_eq!(
            self.status,
            FiberStatus::Running,
            "Called run on Fiber with a status that is not running."
        );
        while self.status == FiberStatus::Running && num_instructions > 0 {
            num_instructions -= 1;
            let (instruction, num_bytes_consumed) =
                Instruction::parse(&self.byte_code[self.ip as usize..])
                    .expect("Couldn't parse instruction.");
            debug!("Next instruction: {:?}", &instruction);
            self.ip += num_bytes_consumed as u64;
            self.run_instruction(instruction);
            debug!("Fiber: {:?}", self);

            if self.ip >= self.byte_code.len() as u64 {
                self.status = FiberStatus::Done(match self.stack.pop().unwrap() {
                    StackEntry::AddressInByteCode(_) => panic!("Can only return values."),
                    StackEntry::AddressInHeap(address) => self.export(address),
                });
            }
        }
    }
    fn run_instruction(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::CreateInt(int) => {
                let address = self.create_object(ObjectData::Int(int));
                self.stack.push(StackEntry::AddressInHeap(address));
            }
            Instruction::CreateString(string) => {
                let address = self.create_object(ObjectData::String(string));
                self.stack.push(StackEntry::AddressInHeap(address));
            }
            Instruction::CreateSmallString(string) => {
                let address = self.create_object(ObjectData::String(string));
                self.stack.push(StackEntry::AddressInHeap(address));
            }
            Instruction::CreateSymbol(symbol) => {
                let address = self.create_object(ObjectData::Symbol(symbol));
                self.stack.push(StackEntry::AddressInHeap(address));
            }
            Instruction::CreateMap(len) => {
                let mut key_value_addresses = vec![];
                for _ in 0..(2 * len) {
                    match self.stack.pop().unwrap() {
                        StackEntry::AddressInByteCode(_) => {
                            panic!("Byte code in a Map?!")
                        }
                        StackEntry::AddressInHeap(address) => key_value_addresses.push(address),
                    }
                }
                let mut map = HashMap::new();
                for mut chunk in &key_value_addresses.into_iter().rev().chunks(2) {
                    let key = chunk.next().unwrap();
                    let value = chunk.next().unwrap();
                    assert_eq!(chunk.next(), None);
                    map.insert(key, value);
                }
                let object_address = self.create_object(ObjectData::Map(map));
                self.stack.push(StackEntry::AddressInHeap(object_address));
            }
            Instruction::CreateList(len) => {
                let mut item_addresses = vec![];
                for _ in 0..len {
                    match self.stack.pop().unwrap() {
                        StackEntry::AddressInByteCode(_) => {
                            panic!("Byte code in a List?!")
                        }
                        StackEntry::AddressInHeap(address) => item_addresses.push(address),
                    }
                }
                let list = item_addresses.into_iter().rev().collect();
                let object_address = self.create_object(ObjectData::List(list));
                self.stack.push(StackEntry::AddressInHeap(object_address));
            }
            Instruction::CreateClosure(num_captured_vars) => {
                let mut captured_vars = vec![];
                for _ in 0..num_captured_vars {
                    match self.stack.pop().unwrap() {
                        StackEntry::AddressInByteCode(_) => {
                            panic!("Closure captures byte code?!")
                        }
                        StackEntry::AddressInHeap(address) => captured_vars.push(address),
                    }
                }
                let captured_vars = captured_vars.into_iter().rev().collect();
                let body_address = match self.stack.pop().unwrap() {
                    StackEntry::AddressInByteCode(address) => address,
                    StackEntry::AddressInHeap(_) => {
                        panic!("Closure captures byte code?!")
                    }
                };
                let object_address = self.create_object(ObjectData::Closure {
                    captured: captured_vars,
                    body: body_address,
                });
                self.stack.push(StackEntry::AddressInHeap(object_address));
            }
            Instruction::Dup(stack_offset) => {
                let address = match self.get_from_stack(stack_offset) {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                self.dup(address);
            }
            Instruction::DupNear(stack_offset) => {
                let address = match self.get_from_stack(stack_offset as StackOffset) {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                self.dup(address);
            }
            Instruction::Drop(stack_offset) => {
                let address = match self.get_from_stack(stack_offset) {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                self.drop(address);
            }
            Instruction::DropNear(stack_offset) => {
                let address = match self.get_from_stack(stack_offset as StackOffset) {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                self.drop(address);
            }
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::PopMultipleBelowTop(n) => {
                let top = self.stack.pop().unwrap();
                for _ in 0..n {
                    self.stack.pop();
                }
                self.stack.push(top);
            }
            Instruction::PushAddress(address) => {
                self.stack.push(StackEntry::AddressInByteCode(address));
            }
            Instruction::PushFromStack(offset) => {
                let entry = self.get_from_stack(offset);
                self.stack.push(entry)
            }
            Instruction::PushNearFromStack(offset) => {
                let entry = self.get_from_stack(offset as StackOffset);
                self.stack.push(entry)
            }
            Instruction::Jump(address) => self.ip = address,
            Instruction::Call => {
                let arg = self.stack.pop().unwrap();
                let closure_address = match self.stack.pop().unwrap() {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                self.stack.push(StackEntry::AddressInByteCode(self.ip));
                let (captured, body) = match self.heap.get(&closure_address).unwrap().data.clone() {
                    ObjectData::Closure { captured, body } => (captured, body),
                    _ => panic!(),
                };
                for object in captured {
                    self.stack.push(StackEntry::AddressInHeap(object));
                }
                self.stack.push(arg);
                self.ip = body;
            }
            Instruction::Return => {
                let return_value = self.stack.pop().unwrap();
                let original_address = match self.stack.pop().unwrap() {
                    StackEntry::AddressInByteCode(address) => address,
                    StackEntry::AddressInHeap(_) => panic!(),
                };
                self.stack.push(return_value);
                self.ip = original_address;
            }
            Instruction::Primitive(kind) => {
                let arg = match self.stack.pop().unwrap() {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                let arg = self.export(arg);

                fn split_kind_and_arg(arg: Value) -> Result<(PrimitiveKind, Value), String> {
                    let (symbol, arg) = arg
                        .list()
                        .needed("Primitive called with a non-list.")?
                        .tuple2()
                        .needed("Primitive called with a list that doesn't contain 2 items.")?;
                    let symbol = symbol
                        .symbol()
                        .needed("Primitive called, but the first argument is not a symbol.")?;
                    let kind = PrimitiveKind::parse(&symbol)
                        .needed(format!("Unknown primitive {}.", symbol))?;
                    Ok((kind, arg))
                }
                let (kind, arg) = match kind {
                    Some(kind) => (kind, arg),
                    None => match split_kind_and_arg(arg) {
                        Ok(kind_and_arg) => kind_and_arg,
                        Err(error) => {
                            self.status = wrong_usage_panic_status(error);
                            return;
                        }
                    },
                };

                let primitive_result = match kind {
                    PrimitiveKind::Add => self.primitive_add(arg).map(Some),
                    PrimitiveKind::GetItem => self.primitive_get_item(arg).map(Some),
                    PrimitiveKind::GetAmbient => self.primitive_get_ambient(arg).map(Some),
                    PrimitiveKind::Panic => self.primitive_panic(arg).map(|_| None),
                    PrimitiveKind::CreateChannel => {
                        self.primitive_create_channel(arg).map(|_| None)
                    }
                    PrimitiveKind::Send => self.primitive_send(arg).map(|_| None),
                    PrimitiveKind::Receive => self.primitive_receive(arg).map(|_| None),
                };
                match primitive_result {
                    Ok(value) => {
                        if let Some(value) = value {
                            let address = self.import(value);
                            self.stack.push(StackEntry::AddressInHeap(address));
                        }
                    }
                    Err(message) => self.status = wrong_usage_panic_status(message),
                }
            }
        }
    }

    // Primitives.

    fn primitive_add(&mut self, arg: Value) -> Result<Value, String> {
        let list = arg.list().needed("add needs a list")?;
        let mut ints = vec![];
        for item in list {
            ints.push(
                item.int()
                    .needed("add needs a list that contains only numbers")?,
            );
        }
        let sum = ints.into_iter().fold(0, |a, b| a + b);
        Ok(Value::Int(sum))
    }

    fn primitive_get_item(&mut self, arg: Value) -> Result<Value, String> {
        let list = arg.list().needed("get-item needs a list")?;
        let (list, index) = list
            .tuple2()
            .needed("get-item needs a list with two values")?;
        let list = list
            .list()
            .needed("get-item needs a list with a list and an index")?;
        let index = index
            .int()
            .needed("get-item needs a list with a list and an index")?;
        if index < 0 || index >= list.len() as i64 {
            return Err(format!(
                "get-item received list {} and index {}, so the index is out of bounds",
                Value::List(list),
                index
            ));
        }
        let item = list[index as usize].clone();
        Ok(item)
    }

    fn primitive_get_ambient(&mut self, arg: Value) -> Result<Value, String> {
        let symbol = arg.symbol().needed("GetAmbient needs a symbol.")?;
        Ok(self
            .ambients
            .get(&symbol)
            .needed(format!("No ambient named {} exists.", symbol))?
            .clone())
    }

    fn primitive_panic(&mut self, arg: Value) -> Result<(), String> {
        self.status = FiberStatus::Panicked(arg);
        Ok(())
    }

    fn primitive_create_channel(&mut self, arg: Value) -> Result<(), String> {
        let capacity = arg.int().needed("CreateChannel needs an int.")?;
        self.status = FiberStatus::CreatingChannel(capacity as u64);
        Ok(())
    }

    fn primitive_send(&mut self, arg: Value) -> Result<(), String> {
        let list = arg.list().needed("Send needs a list.")?;
        let (channel_end, message) = list
            .tuple2()
            .needed("Send called with a list that has a different number than 2 elements.")?;
        let channel_end = channel_end
            .channel_send_end()
            .needed("Send called with a list where the first item is not a ChannelSendEnd.")?;
        self.status = FiberStatus::Sending(channel_end, message);
        Ok(())
    }

    fn primitive_receive(&mut self, arg: Value) -> Result<(), String> {
        let channel_end = arg
            .channel_receive_end()
            .needed("Receive called, but the argument is not a ChannelReceiveEnd.")?;
        self.status = FiberStatus::Receiving(channel_end);
        Ok(())
    }

    // Resolve status.

    pub fn resolve_creating_channel(&mut self, id: ChannelId) {
        let address = self.import(Value::List(vec![
            Value::ChannelSendEnd(id),
            Value::ChannelReceiveEnd(id),
        ]));
        self.stack.push(StackEntry::AddressInHeap(address));
        self.status = FiberStatus::Running;
    }

    pub fn resolve_sending(&mut self) {
        let address = self.import(Value::unit());
        self.stack.push(StackEntry::AddressInHeap(address));
        self.status = FiberStatus::Running;
    }

    pub fn resolve_receiving(&mut self, message: Value) {
        let address = self.import(message);
        self.stack.push(StackEntry::AddressInHeap(address));
        self.status = FiberStatus::Running;
    }
}

trait Needed<T> {
    fn needed<I: Into<String>>(self, message: I) -> Result<T, String>;
}
impl<T> Needed<T> for Option<T> {
    fn needed<I: Into<String>>(self, message: I) -> Result<T, String> {
        match self {
            Some(value) => Ok(value),
            None => Err(message.into()),
        }
    }
}

fn wrong_usage_panic_status<I: Into<String>>(message: I) -> FiberStatus {
    FiberStatus::Panicked(Value::List(vec![
        Value::Symbol("wrong-usage".into()),
        Value::String(message.into()),
    ]))
}
