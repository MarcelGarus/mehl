use log::debug;

use crate::utils::RemoveLast;

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
        // println!("Compiling body. Captured: {:?}", self.captured);
        stack.push(self.in_);
        for statement in &self.code {
            // println!("Stack is {:?}.", stack);
            statement.compile(out, &mut stack);
        }
        out.push_instruction(push_from_stack_instruction(stack.reference_id(self.out)));
        out.push_instruction(Instruction::PopMultipleBelowTop(
            stack.len().try_into().unwrap(),
        ));
        stack.clear();
        stack.push(self.out);
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
                    out.push_instruction(if string.len() < 256 {
                        CreateSmallString(string.clone())
                    } else {
                        CreateString(string.clone())
                    });
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
                    stack.push(999999999);
                    for id in &closure.captured {
                        out.push_instruction(push_from_stack_instruction(stack.reference_id(*id)));
                        stack.push(*id);
                    }
                    out.push_instruction(CreateClosure(closure.captured.len() as u64));
                    for _ in 0..(closure.captured.len() + 1) {
                        stack.remove_last();
                    }
                    stack.push(*id);
                }
                lir::Expr::Map(map) => {
                    for (key, value) in map {
                        out.push_instruction(push_from_stack_instruction(stack.reference_id(*key)));
                        stack.push(*key);
                        out.push_instruction(push_from_stack_instruction(
                            stack.reference_id(*value),
                        ));
                        stack.push(*value);
                    }
                    out.push_instruction(CreateMap(map.len() as u64));
                    for _ in map {
                        stack.remove_last();
                        stack.remove_last();
                    }
                    stack.push(*id);
                }
                lir::Expr::List(list) => {
                    for item in list {
                        out.push_instruction(push_from_stack_instruction(
                            stack.reference_id(*item),
                        ));
                        stack.push(*item);
                    }
                    out.push_instruction(CreateList(list.len() as u64));
                    for _ in list {
                        stack.remove_last();
                    }
                    stack.push(*id);
                }
                lir::Expr::Call { closure, arg } => {
                    out.push_instruction(push_from_stack_instruction(stack.reference_id(*closure)));
                    stack.push(*closure);
                    out.push_instruction(push_from_stack_instruction(stack.reference_id(*arg)));
                    stack.push(*arg);
                    out.push_instruction(Call);
                    stack.remove_last();
                    stack.remove_last();
                    stack.push(*id);
                }
                lir::Expr::Primitive { kind, arg } => {
                    out.push_instruction(push_from_stack_instruction(stack.reference_id(*arg)));
                    stack.push(*arg);
                    out.push_instruction(Instruction::Primitive(*kind));
                    stack.remove_last();
                    stack.push(*id);
                }
            },
            lir::Statement::Dup(ids) => {
                for id in ids {
                    out.push_instruction(dup_instruction(stack.reference_id(*id)))
                }
            }
            lir::Statement::Drop(ids) => {
                for id in ids {
                    out.push_instruction(drop_instruction(stack.reference_id(*id)))
                }
            }
        }
    }
}

fn dup_instruction(offset: StackOffset) -> Instruction {
    if offset < 256 {
        Instruction::DupNear(offset as u8)
    } else {
        Instruction::Dup(offset)
    }
}
fn drop_instruction(offset: StackOffset) -> Instruction {
    if offset < 256 {
        Instruction::DropNear(offset as u8)
    } else {
        Instruction::Drop(offset)
    }
}
fn push_from_stack_instruction(offset: StackOffset) -> Instruction {
    if offset < 256 {
        Instruction::PushNearFromStack(offset as u8)
    } else {
        Instruction::PushFromStack(offset)
    }
}

pub trait ByteCodeExt {
    fn current_address(&self) -> Address;
    fn push_instruction(&mut self, instruction: Instruction);
    fn update_jump_target(&mut self, jump_address: Address, target: Address);
}
impl ByteCodeExt for ByteCode {
    fn current_address(&self) -> Address {
        self.len() as u64
    }
    fn push_instruction(&mut self, instruction: Instruction) {
        debug!("Pushing instruction {:?}", instruction);
        self.append(&mut instruction.to_bytes());
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
        // println!("Finding {} in stack model ({:?})", id, self);
        self.iter().rev().position(|it| *it == id).unwrap() as StackOffset
    }
}
