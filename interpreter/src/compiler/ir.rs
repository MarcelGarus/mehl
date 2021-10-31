use itertools::Itertools;
use std::{collections::HashMap, fmt};

pub type Id = u32;

#[derive(Clone, PartialEq, Eq)]
pub enum Statement {
    Int(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Id, Id>),
    List(Vec<Id>),
    Code {
        in_: Id,
        out: Id,
        statements: Statements,
    },
    Call {
        fun: Id,
        arg: Id,
    },
    Primitive {
        arg: Id,
    },
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Statements {
    statements: Vec<(Id, Statement)>,
}

pub struct Ir {
    pub statements: Statements,
    pub out: Id,
}

impl std::hash::Hash for Statement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Statement::Int(int) => int.hash(state),
            Statement::String(string) => string.hash(state),
            Statement::Symbol(symbol) => symbol.hash(state),
            Statement::Map(map) => {
                let mut hash = 0;
                for (key, value) in map {
                    hash ^= key;
                    hash ^= value;
                }
                hash.hash(state);
            }
            Statement::List(list) => list.hash(state),
            Statement::Code {
                in_,
                out,
                statements,
            } => {
                in_.hash(state);
                out.hash(state);
                statements.hash(state);
            }
            Statement::Call { fun, arg } => {
                fun.hash(state);
                arg.hash(state);
            }
            Statement::Primitive { arg } => {
                arg.hash(state);
            }
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Int(int) => write!(f, "int {}", int),
            Statement::String(string) => write!(f, "string {:?}", string),
            Statement::Symbol(symbol) => write!(f, "symbol :{}", symbol),
            Statement::Map(map) => write!(f, "map {:?}", map),
            Statement::List(list) => write!(f, "list {:?}", list),
            Statement::Code {
                in_,
                out,
                statements,
            } => {
                write!(
                    f,
                    "code [\n  in: {}\n{}  out: {}\n]",
                    in_,
                    statements
                        .to_string()
                        .lines()
                        .map(|line| format!("  {}\n", line))
                        .join(""),
                    out
                )
            }
            Statement::Call { fun, arg } => write!(f, "call {}({})", fun, arg),
            Statement::Primitive { arg } => write!(f, "primitive {}", arg),
        }
    }
}
impl fmt::Display for Statements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (id, action) in &self.statements {
            write!(f, "{} = {}\n", id, action)?;
        }
        Ok(())
    }
}

impl fmt::Display for Ir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}out: {}", self.statements, self.out)?;
        Ok(())
    }
}

impl Statements {
    pub fn new() -> Self {
        Self { statements: vec![] }
    }
    pub fn push(&mut self, id: Id, statement: Statement) {
        self.statements.push((id, statement))
    }
    pub fn remove(&mut self, id: Id) {
        let index = self
            .statements
            .iter()
            .position(|(the_id, _)| *the_id == id)
            .unwrap();
        self.statements.remove(index);
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(Id, Statement)> {
        self.statements.iter()
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut (Id, Statement)> {
        self.statements.iter_mut()
    }
}
