use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Ast {
    Number(i64),
    String(String),
    Symbol(String),
    Name(String),
    List(Vec<Code>),
    Map(HashMap<String, Ast>),
    Code(Code),
}
type Code = Vec<Ast>;
impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ast::Number(number) => write!(f, "{}", number),
            Ast::String(string) => write!(f, "{:?}", string),
            Ast::Symbol(symbol) => write!(f, ":{}", symbol),
            Ast::Name(name) => write!(f, "{}", name),
            Ast::List(list) => write!(
                f,
                "({})",
                itertools::join(
                    list.iter().map(|expression| itertools::join(
                        expression.iter().map(|blub| format!("{}", blub)),
                        " "
                    )),
                    ", ",
                )
            ),
            Ast::Map(map) => write!(f, "{{{:?}}}", map),
            Ast::Code(code) => write!(
                f,
                "[{}]",
                itertools::join(code.iter().map(|expression| format!("{}", expression)), " ",)
            ),
        }
    }
}

impl Ast {
    pub fn contains_dot_literal(&self) -> bool {
        match self {
            Ast::Name(name) => name == ".",
            Ast::List(items) => items
                .iter()
                .any(|item| item.iter().any(|ast| ast.contains_dot_literal())),
            Ast::Map(map) => map.values().any(|item| item.contains_dot_literal()),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Number(i64),
    String(String),
    Symbol(String),
    List(Vec<Expr>),
    Map(HashMap<String, Expr>),
    Code(Vec<Ast>),
}
impl Expr {
    pub fn unit() -> Self {
        Expr::Symbol("".into())
    }
}
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(number) => write!(f, "{:?}", number),
            Expr::String(string) => write!(f, "{:?}", string),
            Expr::Symbol(symbol) => write!(f, ":{}", symbol),
            Expr::List(list) => write!(
                f,
                "({})",
                itertools::join(
                    list.iter().map(|expression| format!("{}", expression)),
                    ", ",
                )
            ),
            Expr::Map(map) => write!(
                f,
                "{{{}}}",
                itertools::join(map.iter().map(|(k, v)| format!(":{} {}", k, v)), ", ",)
            ),
            Expr::Code(code) => write!(
                f,
                "[{}]",
                itertools::join(
                    code.iter().map(|expression| format!("{}", expression)),
                    ", ",
                )
            ),
        }
    }
}
