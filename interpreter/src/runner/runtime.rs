use crate::ast::*;
use im::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Default)]
pub struct Runtime {
    depth: u64,
    next_context_id: u64,
}
impl Runtime {
    pub fn next_context_id(&mut self) -> u64 {
        let id = self.next_context_id;
        self.next_context_id += 1;
        id
    }
    pub fn log(&mut self, msg: &str) {
        // println!("{}{}", "  ".repeat(self.depth as usize), msg);
    }

    pub fn depth_increase(&mut self) {
        self.depth += 1;
    }
    pub fn depth_decrease(&mut self) {
        self.depth -= 1;
    }

    pub fn print(&mut self, expr: &Expr) {
        println!("🌮> {}", expr);
    }
    pub fn wait(&mut self, seconds: u64) {
        std::thread::sleep(std::time::Duration::new(seconds, 0));
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Expr {
    Number(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Expr, Expr>),
    List(Vec<Expr>),
    Code { scope: Box<Context>, asts: Asts },
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
            Expr::Code { scope, asts } => (scope.id, asts).hash(state),
        }
    }
}

impl Expr {
    pub fn unit() -> Self {
        Self::Symbol("".into())
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
            Expr::Code { scope: _, asts } => {
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

#[derive(Clone)]
pub struct Context {
    id: u64,
    pub funs: HashMap<String, Fun>,
    pub dot: Expr,
}
impl Eq for Context {}
impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Context {
    pub fn root(runtime: &mut Runtime) -> Self {
        let mut funs = HashMap::new();
        funs.insert(
            "✨".into(),
            Fun {
                name: "✨".into(),
                docs: Some("The primitive fun.".into()),
                body: FunBody::Primitive,
                export_level: 0,
            },
        );
        Self {
            id: runtime.next_context_id(),
            funs,
            dot: Expr::unit(),
        }
    }

    pub fn next(self, runtime: &mut Runtime, dot: Expr) -> Self {
        Self {
            id: runtime.next_context_id(),
            funs: self.funs,
            dot,
        }
    }
}

#[derive(Clone)]
pub struct Fun {
    pub name: String,
    pub docs: Option<String>,
    pub body: FunBody,
    pub export_level: u16,
}
#[derive(Clone)]
pub enum FunBody {
    Primitive,
    Code { scope: Rc<Context>, body: Rc<Asts> },
    Value(Rc<Expr>),
}
