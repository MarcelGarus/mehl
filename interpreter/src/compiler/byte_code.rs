use itertools::Itertools;

use super::primitives::PrimitiveKind;
use std::convert::TryInto;

/// The byte code is just a binary representations of `Instruction`s.
pub type ByteCode = Vec<u8>;

/// An address in the byte code, relative to the start of the byte code.
pub type Address = u64;

/// A relative reference to a stack entry.
pub type StackOffset = u64;
pub type NearStackOffset = u8;

#[derive(Debug)]
pub enum Instruction {
    // Creation of literal objects.
    CreateInt(i64),            // Creates an Int.
    CreateString(String),      // Creates a String.
    CreateSmallString(String), // Creates a String that is at most 255 characters long.
    CreateSymbol(String),      // Creates a Symbol.
    CreateMap(u64),            // key | value | ... | key | value | CreateMap(len)
    CreateList(u64),           // item | item | ... | item | CreateList(len)
    CreateClosure(u64), // ip | captured_var | ... | captured_var | CreateClosure(num_captured_vars)

    // Reference counting.
    Dup(StackOffset),          // Increases the refcount of the object by one.
    DupNear(NearStackOffset),  // Like `Dup`.
    Drop(StackOffset),         // Decreases the refcount of the object by one and frees it on 0.
    DropNear(NearStackOffset), // Like `Drop`.

    // Stack manipulation.
    Pop,                                // Pops a stack value.
    PopMultipleBelowTop(u8), // Leaves the top-most stack item untouched, but removes n below.
    PushAddress(Address),    // Pushes an address that points into the bytecode.
    PushFromStack(StackOffset), // Pushes a value from back in the stack on the stack again.
    PushNearFromStack(NearStackOffset), // Like `PushFromStack`.

    // Control flow.
    Jump(Address),
    Call,   // closure | arg | Call
    Return, // Returns from the current closure to the original IP.

    // Primitives.
    Primitive(Option<PrimitiveKind>),
}

/// Conversion from instructions to byte code.
impl Instruction {
    pub fn to_bytes(&self) -> Vec<u8> {
        use Bytes::*;
        use Instruction::*;
        let quasi_bytes = match self {
            // 0X: Value creation instructions.
            CreateInt(int) => vec![U8(0), U64(*int as u64)],
            CreateString(string) => vec![U8(1), U64(string.len() as u64)]
                .into_iter()
                .chain(string.bytes().map(|byte| U8(byte)))
                .collect_vec(),
            CreateSmallString(string) => vec![U8(2), U8(string.len() as u8)]
                .into_iter()
                .chain(string.bytes().map(|byte| U8(byte)))
                .collect_vec(),
            CreateSymbol(symbol) => vec![U8(3)]
                .into_iter()
                .chain(symbol.bytes().map(|byte| U8(byte)))
                .chain(vec![U8(0)].into_iter())
                .collect_vec(),
            CreateMap(len) => vec![U8(4), U64(*len)],
            CreateList(len) => vec![U8(5), U64(*len)],
            CreateClosure(num_captured_vars) => vec![U8(6), U64(*num_captured_vars)],
            // 1X: Reference counting.
            Dup(offset) => vec![U8(10), U64(*offset)],
            DupNear(offset) => vec![U8(11), U8(*offset)],
            Drop(offset) => vec![U8(12), U64(*offset)],
            DropNear(offset) => vec![U8(13), U8(*offset)],
            // 2X: Stack manipulation.
            Pop => vec![U8(20)],
            PopMultipleBelowTop(num) => vec![U8(21), U8(*num)],
            PushAddress(addr) => vec![U8(22), U64(*addr)],
            PushFromStack(offset) => vec![U8(23), U64(*offset)],
            PushNearFromStack(offset) => vec![U8(24), U8(*offset)],
            // 3X: Control flow.
            Jump(addr) => vec![U8(30), U64(*addr)],
            Call => vec![U8(31)],
            Return => vec![U8(32)],
            // >=100: Primitives.
            Primitive(None) => vec![U8(100)],
            Primitive(Some(kind)) => {
                vec![U8(101
                    + PrimitiveKind::all()
                        .iter()
                        .position(|it| it == kind)
                        .unwrap() as u8)]
            }
        };
        let mut bytes = vec![];
        for quasi_byte in quasi_bytes {
            quasi_byte.out(&mut bytes);
        }
        bytes
    }
}
enum Bytes {
    U8(u8),
    U64(u64),
}
impl Bytes {
    fn out(self, out: &mut Vec<u8>) {
        match self {
            Bytes::U8(u8) => out.push(u8),
            Bytes::U64(u64) => {
                for byte in u64.to_le_bytes() {
                    out.push(byte);
                }
            }
        }
    }
}

/// Conversion from byte code to instructions.
impl Instruction {
    // TODO: Handle errors better.
    pub fn parse(byte_code: &[u8]) -> Result<(Self, u8), ()> {
        let mut parser = Parser(byte_code);
        let instruction = parser.parse();
        let num_bytes_consumed = parser.0.as_ptr() as usize - byte_code.as_ptr() as usize;
        Ok((instruction, num_bytes_consumed as u8))
    }
}
struct Parser<'a>(&'a [u8]);
impl<'a> Parser<'a> {
    fn get_u8(&mut self) -> u8 {
        let value = self.0[0];
        self.0 = &self.0[1..];
        value
    }
    fn get_u64(&mut self) -> u64 {
        let value = u64::from_le_bytes(self.0[0..8].try_into().unwrap());
        self.0 = &self.0[8..];
        value
    }
    fn parse(&mut self) -> Instruction {
        use Instruction::*;
        match self.get_u8() {
            0 => CreateInt(self.get_u64() as i64),
            1 => {
                let len = self.get_u64();
                let string =
                    String::from_utf8(self.0[..len as usize].to_vec()).expect("Invalid UTF8.");
                self.0 = &self.0[len as usize..];
                CreateString(string)
            }
            2 => {
                let len = self.get_u8();
                let string =
                    String::from_utf8(self.0[..len as usize].to_vec()).expect("Invalid UTF8.");
                self.0 = &self.0[len as usize..];
                CreateSmallString(string)
            }
            3 => {
                let mut bytes = vec![];
                loop {
                    match self.get_u8() {
                        0 => break,
                        n => bytes.push(n),
                    }
                }
                let symbol = String::from_utf8(bytes).expect("Invalid UTF8.");
                CreateSymbol(symbol)
            }
            4 => CreateMap(self.get_u64()),
            5 => CreateList(self.get_u64()),
            6 => CreateClosure(self.get_u64()),
            10 => Dup(self.get_u64()),
            11 => DupNear(self.get_u8()),
            12 => Drop(self.get_u64()),
            13 => DropNear(self.get_u8()),
            20 => Pop,
            21 => PopMultipleBelowTop(self.get_u8()),
            22 => PushAddress(self.get_u64()),
            23 => PushFromStack(self.get_u64()),
            24 => PushNearFromStack(self.get_u8()),
            30 => Jump(self.get_u64()),
            31 => Call,
            32 => Return,
            100 => Primitive(None),
            opcode => {
                let kinds = PrimitiveKind::all();
                if opcode < 101 || opcode > 100 + kinds.len() as u8 {
                    panic!("Unknown byte code opcode {}.", opcode);
                }
                Primitive(Some(kinds[(opcode - 101) as usize]))
            }
        }
    }
}
