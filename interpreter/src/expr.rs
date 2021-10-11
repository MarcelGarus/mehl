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
        let mut map = self.as_map()?;
        let mut list = vec![];
        let length = map.remove(&Expr::Symbol("length".into()))?.as_number()?;
        for i in 0..length {
            list.push(map.remove(&Expr::Symbol(format!("item-{}", i)))?);
        }
        Some(list)
    }
}
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(number) => write!(f, "{}", number),
            Expr::String(string) => write!(f, "{:?}", string),
            Expr::Symbol(symbol) => write!(f, ":{}", symbol),
            Expr::Map(map) => {
                write!(f, "{{")?;
                for (key, value) in map {
                    write!(f, "{}, {}, ", key, value)?;
                }
                write!(f, "}}")?;
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
        }
    }
}

impl Ast {
    pub fn to_expr(self) -> Expr {
        match self {
            Ast::Number(number) => Expr::Number(number),
            Ast::String(string) => Expr::String(string),
            Ast::Symbol(symbol) => Expr::Symbol(symbol),
            Ast::Map(map) => {
                let mut new_map = HashMap::new();
                for (key, value) in map {
                    new_map.insert(key.to_expr(), value.to_expr());
                }
                Expr::Map(new_map)
            }
            Ast::List(list) => {
                let mut map = HashMap::new();
                for (i, item) in list.into_iter().enumerate() {
                    map.insert(Expr::Symbol(format!("item{}", i)), item.to_expr());
                }
                Expr::Map(map)
            }
            Ast::Code(code) => {
                let mut map = HashMap::new();
                for (i, ast) in code.into_iter().enumerate() {
                    map.insert(Expr::Symbol(format!("item-{}", i)), ast.to_quoted_expr());
                }
                Expr::Map(map)
            }
            Ast::Name(_) => panic!("Called to_expr on an AST name."),
        }
    }

    pub fn to_quoted_expr(self) -> Expr {
        match self {
            Ast::Number(number) => {
                let mut map = HashMap::new();
                map.insert(Expr::Symbol("type".into()), Expr::Symbol("number".into()));
                map.insert(Expr::Symbol("value".into()), Expr::Number(number));
                Expr::Map(map)
            }
            Ast::String(string) => {
                let mut map = HashMap::new();
                map.insert(Expr::Symbol("type".into()), Expr::Symbol("string".into()));
                map.insert(Expr::Symbol("value".into()), Expr::String(string));
                Expr::Map(map)
            }
            Ast::Symbol(symbol) => {
                let mut map = HashMap::new();
                map.insert(Expr::Symbol("type".into()), Expr::Symbol("symbol".into()));
                map.insert(Expr::Symbol("value".into()), Expr::Symbol(symbol));
                Expr::Map(map)
            }
            Ast::Map(ast_map) => {
                let mut map = HashMap::new();
                map.insert(Expr::Symbol("type".into()), Expr::Symbol("map".into()));
                let mut i = 0;
                for (key, value) in ast_map {
                    map.insert(Expr::Symbol(format!("key-{}", i)), key.to_quoted_expr());
                    map.insert(Expr::Symbol(format!("value-{}", i)), value.to_quoted_expr());
                    i += 1;
                }
                Expr::Map(map)
            }
            Ast::List(list) => {
                let mut map = HashMap::new();
                map.insert(Expr::Symbol("type".into()), Expr::Symbol("list".into()));
                for (i, item) in list.into_iter().enumerate() {
                    map.insert(Expr::Symbol(format!("item-{}", i)), item.to_quoted_expr());
                }
                Expr::Map(map)
            }
            Ast::Code(code) => {
                let mut map = HashMap::new();
                map.insert(Expr::Symbol("type".into()), Expr::Symbol("code".into()));
                for (i, item) in code.into_iter().enumerate() {
                    map.insert(Expr::Symbol(format!("item-{}", i)), item.to_quoted_expr());
                }
                Expr::Map(map)
            }
            Ast::Name(name) => {
                let mut map = HashMap::new();
                map.insert(Expr::Symbol("type".into()), Expr::Symbol("name".into()));
                map.insert(Expr::Symbol("value".into()), Expr::Symbol(name));
                Expr::Map(map)
            }
        }
    }
}
