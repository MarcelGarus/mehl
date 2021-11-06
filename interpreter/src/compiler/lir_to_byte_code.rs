use super::byte_code::*;
use super::lir;
use super::lir::Id;
use std::convert::TryInto;

impl lir::Closure {
    pub fn compile_to_byte_code(&self) -> ByteCode {
        let mut byte_code = vec![];
        self.compile_body(&mut byte_code);
        byte_code
    }

    fn compile_body(&self, out: &mut ByteCode) {
        let mut stack = self.captured.clone();
        stack.push(self.in_);
        for statement in &self.code {
            statement.compile(out, &mut stack);
        }
        out.push_instruction(Instruction::PushFromStack(stack.reference_id(self.out)));
        out.push_instruction(Instruction::PopMultipleBelowTop(
            stack.len().try_into().unwrap(),
        ));
    }
}

impl lir::Statement {
    fn compile(&self, out: &mut ByteCode, stack: &mut StackModel) {
        use Instruction::*;
        match self {
            lir::Statement::Assignment { id, value } => match value {
                lir::Expr::Int(int) => {
                    out.push_instruction(CreateInt(*int));
                    stack.push(*id);
                }
                lir::Expr::String(string) => {
                    out.push_instruction(CreateString(string.clone()));
                    stack.push(*id);
                }
                lir::Expr::Symbol(symbol) => {
                    out.push_instruction(CreateSymbol(symbol.clone()));
                    stack.push(*id);
                }
                lir::Expr::Closure(closure) => {
                    let jump_addr = out.current_address();
                    out.push_instruction(Jump(0)); // The target will be updated later.
                    let body_addr = out.current_address();
                    closure.compile_body(out);
                    let after_body_addr = out.current_address();
                    out.update_jump_target(jump_addr, after_body_addr);
                    out.push_instruction(PushAddress(body_addr));
                    for id in &closure.captured {
                        out.push_instruction(PushFromStack(stack.reference_id(*id)));
                    }
                    out.push_instruction(CreateClosure(closure.captured.len() as u64));
                    stack.push(*id);
                }
                lir::Expr::Map(map) => {
                    for (key, value) in map {
                        out.push_instruction(PushFromStack(stack.reference_id(*key)));
                        out.push_instruction(PushFromStack(stack.reference_id(*value)));
                    }
                    out.push_instruction(CreateMap(map.len() as u64));
                    stack.push(*id);
                }
                lir::Expr::List(list) => {
                    for item in list {
                        out.push_instruction(PushFromStack(stack.reference_id(*item)));
                    }
                    out.push_instruction(CreateList(list.len() as u64));
                    stack.push(*id);
                }
                lir::Expr::Call { closure, arg } => {
                    out.push_instruction(PushFromStack(stack.reference_id(*closure)));
                    out.push_instruction(PushFromStack(stack.reference_id(*arg)));
                    out.push_instruction(Call);
                    stack.push(*id);
                }
                lir::Expr::Primitive { kind, arg } => {
                    out.push_instruction(PushFromStack(stack.reference_id(*arg)));
                    use super::hir::Primitive::*;
                    out.push_instruction(match *kind {
                        Magic => Instruction::Primitive,
                        Print => Instruction::PrimitivePrint,
                        _ => panic!("Unknown primitive {:?}.", kind),
                    });
                    stack.push(*id);
                }
            },
            lir::Statement::Dup(id) => out.push_instruction(Dup(stack.reference_id(*id))),
            lir::Statement::Drop(id) => out.push_instruction(Drop(stack.reference_id(*id))),
        }
    }
}

pub trait ByteCodeExt {
    fn current_address(&self) -> Address;
    fn push_u8(&mut self, u8: u8);
    fn push_u64(&mut self, u64: u64);
    fn push_instruction(&mut self, instruction: Instruction);
    fn update_jump_target(&mut self, jump_address: Address, target: Address);
}
impl ByteCodeExt for ByteCode {
    fn current_address(&self) -> Address {
        self.len() as u64
    }
    fn push_u8(&mut self, u8: u8) {
        self.push(u8);
    }
    fn push_u64(&mut self, u64: u64) {
        for byte in &u64.to_le_bytes() {
            self.push_u8(*byte);
        }
    }
    fn push_instruction(&mut self, instruction: Instruction) {
        use Instruction::*;
        match instruction {
            CreateInt(int) => {
                self.push_u8(0);
                self.push_u64(int as u64);
            }
            CreateString(string) => {
                self.push_u8(1);
                self.push_u64(string.len() as u64);
                for byte in string.bytes() {
                    self.push_u8(byte);
                }
            }
            CreateSymbol(symbol) => {
                self.push_u8(2);
                for byte in symbol.bytes() {
                    self.push_u8(byte);
                }
                self.push_u8(0);
            }
            CreateMap(len) => {
                self.push_u8(3);
                self.push_u64(len);
            }
            CreateList(len) => {
                self.push_u8(4);
                self.push_u64(len);
            }
            CreateClosure(num_captured_vars) => {
                self.push_u8(5);
                self.push_u64(num_captured_vars);
            }
            Dup(offset) => {
                self.push_u8(6);
                self.push_u64(offset);
            }
            Drop(offset) => {
                self.push_u8(7);
                self.push_u64(offset);
            }
            Pop => self.push_u8(8),
            PopMultipleBelowTop(num) => {
                self.push_u8(9);
                self.push_u8(num);
            }
            PushAddress(addr) => {
                self.push_u8(10);
                self.push_u64(addr);
            }
            PushFromStack(offset) => {
                self.push_u8(11);
                self.push_u64(offset);
            }
            Jump(addr) => {
                self.push_u8(12);
                self.push_u64(addr);
            }
            Call => self.push_u8(13),
            Return => self.push_u8(14),
            Primitive => self.push_u8(15),
            PrimitivePrint => self.push_u8(16),
        }
    }
    fn update_jump_target(&mut self, jump_address: Address, target: Address) {
        let jump_address = jump_address as usize;
        let target = target.to_le_bytes();
        for i in 0..8 {
            self[jump_address + 1 + i] = target[i];
        }
    }
}

type StackModel = Vec<Id>;
trait StackModelExt {
    fn reference_id(&self, id: Id) -> StackOffset;
}
impl StackModelExt for StackModel {
    fn reference_id(&self, id: Id) -> StackOffset {
        self.iter().rev().position(|it| *it == id).unwrap() as StackOffset
    }
}