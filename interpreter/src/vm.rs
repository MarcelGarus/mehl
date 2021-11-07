use std::collections::HashMap;

use itertools::Itertools;

use crate::compiler::byte_code::{ByteCode, Instruction, StackOffset};

type Pointer = u64;

#[derive(Debug)]
pub struct Vm {
    byte_code: ByteCode,
    ip: Pointer, // instruction pointer
    stack: Vec<StackEntry>,
    heap: HashMap<Pointer, Object>, // TODO: dynamically allocate objects
    next_heap_address: u64,
}

// TODO: Unify both cases and inline some values. The size of the stack is very
// performance-critical, so explicitly controlling its memory layout would be
// nice.
#[derive(Debug, Clone)]
enum StackEntry {
    AddressInByteCode(Pointer),
    AddressInHeap(Pointer),
}

#[derive(Debug)]
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
        body: Pointer,
    },
}

impl Vm {
    pub fn new(byte_code: ByteCode) -> Self {
        Self {
            byte_code,
            ip: 0,
            stack: vec![],
            heap: HashMap::new(),
            next_heap_address: 123450000,
        }
    }

    pub fn run(&mut self) {
        while self.ip < self.byte_code.len() as u64 {
            let (instruction, num_bytes_consumed) =
                Instruction::parse(&self.byte_code[self.ip as usize..])
                    .expect("Couldn't parse instruction.");
            println!("VM: {:?}\nNext instruction: {:?}", &self, &instruction);
            self.execute_instruction(instruction);
            self.ip += num_bytes_consumed as u64;
        }
    }
    fn execute_instruction(&mut self, instruction: Instruction) {
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
                self.heap.get_mut(&address).unwrap().reference_count += 1;
            }
            Instruction::DupNear(stack_offset) => {
                let address = match self.get_from_stack(stack_offset as StackOffset) {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                self.heap.get_mut(&address).unwrap().reference_count += 1;
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
            Instruction::Primitive => {
                let arg = match self.stack.pop().unwrap() {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                let list = match self.heap.get(&arg).unwrap().data.clone() {
                    ObjectData::List(list) => list,
                    _ => panic!("Primitive called with a non-list."),
                };
                if list.len() != 2 {
                    panic!(
                        "Primitive called with a list with {} instead of 2 items.",
                        list.len()
                    );
                }
                let symbol = match self.heap.get(&list[0]).unwrap().data.clone() {
                    ObjectData::Symbol(symbol) => symbol,
                    _ => panic!("Primitive called, but the first argument is not a symbol."),
                };
                let arg = list[1];
                let object = match symbol.as_str() {
                    "print" => self.primitive_print(arg),
                    _ => todo!("Unhandled primitive."),
                };
                let address = self.create_object(object);
                self.stack.push(StackEntry::AddressInHeap(address));
            }
            Instruction::PrimitivePrint => {
                let address = match self.stack.pop().unwrap() {
                    StackEntry::AddressInByteCode(_) => panic!(),
                    StackEntry::AddressInHeap(address) => address,
                };
                let object = self.primitive_print(address);
                let address = self.create_object(object);
                self.stack.push(StackEntry::AddressInHeap(address));
            }
        }
    }

    fn get_from_stack(&self, offset: StackOffset) -> StackEntry {
        self.stack[self.stack.len() - (offset as usize) - 1].clone()
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
    fn drop(&mut self, address: Pointer) {
        let object = self.heap.get_mut(&address).unwrap();
        object.reference_count -= 1;
        if object.reference_count == 0 {
            self.heap.remove(&address); // Free the object.
        }
    }

    // Primitives.
    fn primitive_print(&mut self, address: Pointer) -> ObjectData {
        let object = self.heap.get(&address).unwrap().data.clone();
        println!("🌮> {:?}", object);
        self.drop(address);
        ObjectData::Symbol(":".into())
    }
}
