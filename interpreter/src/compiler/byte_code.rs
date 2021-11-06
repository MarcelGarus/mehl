use super::lir::Id;

/// The byte code is just a binary representations of `Instruction`s.
pub type ByteCode = Vec<u8>;

/// An address in the byte code, relative to the start of the byte code.
pub type Address = u64;

/// A relative reference to a stack entry.
pub type StackOffset = u64;

pub enum Instruction {
    // Creation of literal objects.
    CreateInt(i64),
    CreateString(String),
    CreateSymbol(String),
    CreateMap(u64),     // key | value | ... | key | value | CreateMap(len)
    CreateList(u64),    // item | item | ... | item | CreateList(len)
    CreateClosure(u64), // ip | captured_var | ... | captured_var | CreateClosure(num_captured_vars)

    // Reference counting.
    Dup(StackOffset),  // Increases the refcount of the object by one.
    Drop(StackOffset), // Decreases the refcount of the object by one and frees it on 0.

    // Stack manipulation.
    Pop,                        // Pops a stack value.
    PopMultipleBelowTop(u8),    // Leaves the top-most stack item untouched, but removes n below.
    PushAddress(Address),       // Pushes an address that points into the bytecode.
    PushFromStack(StackOffset), // Pushes a value from back in the stack on the stack again.

    // Control flow.
    Jump(Address),
    Call,   // closure | arg | Call
    Return, // Returns from the current closure to the original IP.

    // Primitives.
    Primitive,
    PrimitivePrint,
}
