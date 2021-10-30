use itertools::Itertools;
use std::{collections::HashMap, fmt};

pub type Id = u32;

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
    // TODO: Specific primitives
}

pub struct Statements {
    statements: Vec<(Id, Statement)>,
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
}
