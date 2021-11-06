use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, fmt};

use itertools::Itertools;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Ast {
    Int(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Asts, Asts>),
    List(Vec<Asts>),
    Code(Asts),
    Name(String),
    Let(String),
    Fun(String),
}
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Asts(Vec<Ast>);
impl Asts {
    pub fn iter(&self) -> impl Iterator<Item = &Ast> {
        self.0.iter()
    }
    pub fn into_iter(self) -> impl Iterator<Item = Ast> {
        self.0.into_iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn into_vec(self) -> Vec<Ast> {
        self.0
    }
}
impl From<Vec<Ast>> for Asts {
    fn from(asts: Vec<Ast>) -> Self {
        Asts(asts)
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ast::Int(int) => write!(f, "{}", int),
            Ast::String(string) => write!(f, "{:?}", string),
            Ast::Symbol(symbol) => write!(f, ":{}", symbol),
            Ast::Map(map) => write!(
                f,
                "{{{}}}",
                itertools::join(
                    map.iter()
                        .map(|(key, value)| format!("{}, {}", &key, &value,)),
                    ", "
                )
            ),
            Ast::List(list) => write!(
                f,
                "({})",
                itertools::join(list.iter().map(|item| item.to_string()), ", ")
            ),
            Ast::Code(code) => write!(
                f,
                "[{}]",
                itertools::join(code.iter().map(|item| item.to_string()), " ")
            ),
            Ast::Name(name) => write!(f, "{}", name),
            Ast::Let(name) => write!(f, "=> {}", name),
            Ast::Fun(name) => write!(f, "-> {}", name),
        }
    }
}

impl fmt::Display for Asts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.iter().map(|ast| ast.to_string()).join(" "))
    }
}

impl Hash for Ast {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Ast::Int(int) => int.hash(state),
            Ast::String(string) => string.hash(state),
            Ast::Symbol(symbol) => symbol.hash(state),
            Ast::Map(map) => {
                let mut h = 0;

                for element in map.iter() {
                    let mut hasher = DefaultHasher::new();
                    element.hash(&mut hasher);
                    h ^= hasher.finish();
                }

                state.write_u64(h);
            }
            Ast::List(list) => list.hash(state),
            Ast::Code(code) => code.hash(state),
            Ast::Name(name) => name.hash(state),
            Ast::Let(name) => name.hash(state),
            Ast::Fun(name) => name.hash(state),
        }
    }
}

pub trait MapGetStrSymbolExt {
    fn get_symbol(&self, key: &str) -> Option<&Ast>;
}
impl MapGetStrSymbolExt for HashMap<Ast, Ast> {
    fn get_symbol(&self, key: &str) -> Option<&Ast> {
        self.get(&Ast::Symbol(key.into()))
    }
}
