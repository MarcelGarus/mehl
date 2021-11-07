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
    Primitive,
    PrimitivePrint,
}

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
            17 => {
                let len = self.get_u8();
                let string =
                    String::from_utf8(self.0[..len as usize].to_vec()).expect("Invalid UTF8.");
                self.0 = &self.0[len as usize..];
                CreateSmallString(string)
            }
            2 => {
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
            3 => CreateMap(self.get_u64()),
            4 => CreateList(self.get_u64()),
            5 => CreateClosure(self.get_u64()),
            6 => Dup(self.get_u64()),
            18 => DupNear(self.get_u8()),
            7 => Drop(self.get_u64()),
            19 => DropNear(self.get_u8()),
            8 => Pop,
            9 => PopMultipleBelowTop(self.get_u8()),
            10 => PushAddress(self.get_u64()),
            11 => PushFromStack(self.get_u64()),
            20 => PushNearFromStack(self.get_u8()),
            12 => Jump(self.get_u64()),
            13 => Call,
            14 => Return,
            15 => Primitive,
            16 => PrimitivePrint,
            opcode => panic!("Unknown byte code opcode {}.", opcode),
        }
    }
}
