pub use super::hir::Id;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

#[derive(Clone, PartialEq, Eq)]
pub enum Statement {
    Assignment { id: Id, value: Expr },
    Dup(Id),
    Drop(Id),
}
#[derive(Clone, PartialEq, Eq)]
pub enum Expr {
    Int(i64),
    String(String),
    Symbol(String),
    Closure(Closure),
    Map(HashMap<Id, Id>),
    List(Vec<Id>),
    Call {
        closure: Id,
        arg: Id,
    },
    Primitive {
        kind: super::hir::Primitive,
        arg: Id,
    },
}
#[derive(Clone, PartialEq, Eq)]
pub struct Closure {
    pub captured: HashSet<Id>,
    pub code: Vec<Statement>,
    pub in_: Id,
    pub out: Id,
}

// in: 0
// 1 = "Hello, world!"   # Creates a new value.
// duplicate 1           # Duplicates the value so it can be used in the closure.
// 2 = closure<1>[       # Inside the closure, 1 needs to be duplicated before use.
//   in: 2               # Ownership of in is transferred into the closure when called.
//   duplicate 1         # Duplicates for usage with print.
//   3 = primitive_print(1) # print consumes the one.
//   drop 2              # At the end of the closure, drop the input.
//   out: 3              # Return the only still owned value.
// ]
// 3 = :
// duplicate 2, 3        # Duplicate 2 and 3 for usage.
// 4 = call 2 3          # Calls closure 2 with arg 3.
// drop 0, 1, 2, 3       # End of the program: Drop all created variables except returned. Closures also drop captured vars.
// out: 4

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Assignment { id, value } => write!(f, "{} = {}", id, value),
            Statement::Dup(id) => write!(f, "dup {}", id),
            Statement::Drop(id) => write!(f, "drop {}", id),
        }
    }
}
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Int(int) => write!(f, "int {}", int),
            Expr::String(string) => write!(f, "string {:?}", string),
            Expr::Symbol(symbol) => write!(f, "symbol :{}", symbol),
            Expr::Map(map) => write!(f, "map {:?}", map),
            Expr::List(list) => write!(f, "list {:?}", list),
            Expr::Closure(closure) => {
                write!(
                    f,
                    "closure<{}>[\n{}\n]",
                    itertools::join(closure.captured.iter(), ", "),
                    itertools::join(
                        closure
                            .to_string()
                            .lines()
                            .map(|line| format!("  {}", line)),
                        "\n"
                    ),
                )
            }
            Expr::Call { closure, arg } => write!(f, "call {}({})", closure, arg),
            Expr::Primitive { kind, arg } => write!(f, "primitive {:?} {}", kind, arg),
        }
    }
}
impl fmt::Display for Closure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "in: {}\n", self.in_)?;
        for statement in &self.code {
            write!(f, "{}\n", statement)?;
        }
        write!(f, "out: {}\n", self.out)?;
        Ok(())
    }
}
