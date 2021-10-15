use crate::ast::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Expr {
    Number(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Expr, Expr>),
    List(Vec<Expr>),
    Code(Asts),
}
impl Expr {
    pub fn unit() -> Self {
        Self::Symbol("".into())
    }
    pub fn as_number(self) -> Option<i64> {
        match self {
            Expr::Number(number) => Some(number),
            _ => None,
        }
    }
    pub fn as_string(self) -> Option<String> {
        match self {
            Expr::String(string) => Some(string),
            _ => None,
        }
    }
    pub fn as_symbol(self) -> Option<String> {
        match self {
            Expr::Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }
    pub fn as_map(self) -> Option<HashMap<Expr, Expr>> {
        match self {
            Expr::Map(map) => Some(map),
            _ => None,
        }
    }
    pub fn as_list(self) -> Option<Vec<Expr>> {
        match self {
            Expr::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn as_code(self) -> Option<Asts> {
        match self {
            Expr::Code(asts) => Some(asts),
            _ => None,
        }
    }
}
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(number) => write!(f, "{}", number),
            Expr::String(string) => write!(f, "{:?}", string),
            Expr::Symbol(symbol) => write!(f, ":{}", symbol),
            Expr::Map(map) => write!(
                f,
                "{{{}}}",
                itertools::join(
                    map.iter().map(|(key, value)| format!("{}, {}", key, value)),
                    ", "
                )
            ),
            Expr::List(list) => write!(
                f,
                "({})",
                itertools::join(list.iter().map(|item| format!("{}", item)), ", ")
            ),
            Expr::Code(asts) => {
                write!(f, "[")?;
                write!(
                    f,
                    "{}, ",
                    itertools::join(asts.iter().map(|ast| format!("{}", ast)), " ")
                )?;
                write!(f, "]")?;
                Ok(())
            }
        }
    }
}
impl Hash for Expr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Expr::Number(number) => number.hash(state),
            Expr::String(string) => string.hash(state),
            Expr::Symbol(symbol) => symbol.hash(state),
            Expr::Map(map) => {
                let mut h = 0;

                for element in map.iter() {
                    let mut hasher = DefaultHasher::new();
                    element.hash(&mut hasher);
                    h ^= hasher.finish();
                }

                state.write_u64(h);
            }
            Expr::List(list) => list.hash(state),
            Expr::Code(asts) => asts.hash(state),
        }
    }
}
