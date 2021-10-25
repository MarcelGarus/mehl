use crate::ast::*;
use im::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub struct Fiber {
    next_context_id: u64,
}
impl Fiber {
    pub fn new() -> Self {
        Self { next_context_id: 0 }
    }
    fn next_context_id(&mut self) -> u64 {
        let id = self.next_context_id;
        self.next_context_id += 1;
        id
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
    pub fn as_code(self) -> Option<(Box<Context>, Asts)> {
        match self {
            Expr::Code { scope, asts } => Some((scope, asts)),
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
    funs: HashMap<String, Rc<Fun>>,
    exported_funs: HashMap<String, Rc<Fun>>,
    pub dot: Expr,
}
impl Eq for Context {}
impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

struct Fun {
    name: String,
    docs: Option<String>,
    scope: Context,
    body: Asts,
}

impl Context {
    pub fn new(fiber: &mut Fiber) -> Self {
        Self {
            id: fiber.next_context_id(),
            funs: HashMap::new(),
            exported_funs: HashMap::new(),
            dot: Expr::unit(),
        }
    }
}

impl Context {
    pub fn run(self, fiber: &mut Fiber, code: Asts) -> Self {
        let mut context = self.clone();
        for ast in code {
            context = context.run_single(fiber, ast);
        }
        context
    }

    fn run_single(self, fiber: &mut Fiber, ast: Ast) -> Self {
        println!(
            "Running {} on {} (known: {:?})",
            format_code(&vec![ast.clone()]),
            self.dot,
            self.funs.keys().collect::<Vec<_>>(),
        );
        match ast {
            Ast::Number(number) => self.next(fiber, Expr::Number(number)),
            Ast::String(string) => self.next(fiber, Expr::String(string)),
            Ast::Symbol(symbol) => self.next(fiber, Expr::Symbol(symbol)),
            Ast::Map(map) => {
                let mut expr_map = HashMap::new();
                let context = self.clone();
                for (key, value) in map {
                    expr_map.insert(
                        context.clone().run(fiber, key).dot,
                        context.clone().run(fiber, value).dot,
                    );
                }
                self.next(fiber, Expr::Map(expr_map))
            }
            Ast::List(list) => {
                let mut expr_list = vec![];
                let context = self.clone();
                for item in list {
                    expr_list.push(context.clone().run(fiber, item).dot);
                }
                self.next(fiber, Expr::List(expr_list))
            }
            Ast::Code(asts) => self.clone().next(
                fiber,
                Expr::Code {
                    scope: Box::new(self),
                    asts,
                },
            ),
            Ast::Name(name) => {
                println!(
                    "Calling fun {}. Known funs: {:?}",
                    &name,
                    self.funs.keys().collect::<Vec<_>>()
                );
                match name.as_str() {
                    "." => self.clone(),
                    "✨" => self.primitive(fiber),
                    name => match self.funs.get(name) {
                        Some(fun) => {
                            println!("Entering fun {}.", name);
                            let result = fun
                                .scope
                                .clone()
                                .next(fiber, self.dot)
                                .run(fiber, fun.body.clone());
                            println!("Exiting fun {}.", name);
                            let mut new_context = Self {
                                dot: result.dot,
                                ..self
                            };
                            for (name, fun) in result.exported_funs {
                                new_context.funs.insert(name, fun);
                            }
                            new_context
                        }
                        None => panic!("Unknown name {}.", name),
                    },
                }
            }
        }
    }

    fn next(self, fiber: &mut Fiber, dot: Expr) -> Self {
        Self {
            id: fiber.next_context_id(),
            funs: self.funs,
            exported_funs: self.exported_funs,
            dot,
        }
    }

    fn primitive(self, fiber: &mut Fiber) -> Self {
        let dot = self.dot.clone();
        let args = match self.dot.clone() {
            Expr::List(args) => args,
            _ => panic!("✨ needs a list, got this: {}", dot),
        };
        if args.len() != 2 {
            panic!("✨ needs a list with two items, got this: {}", dot);
        }
        let name = match &args[0] {
            Expr::Symbol(name) => name,
            _ => panic!(
                "✨ needs a symbol as the first tuple item, got this: {}",
                dot
            ),
        };
        let arg = args[1].clone();
        let context = self.clone().next(fiber, arg.clone());
        match name.as_ref() {
            "export" => context.primitive_export(),
            "fun" => match context.primitive_fun() {
                Ok(context) => context,
                Err(err) => panic!("{}\nDot: {}", err, arg),
            },
            "fun-and-export" => match context.primitive_fun_and_export(fiber) {
                Ok(context) => context,
                Err(err) => panic!("{}\nDot: {}", err, arg),
            },
            // "List[Int].+" => context.primitive_add_list_of_ints(arg),
            // "identical" => context.primitive_identical(arg),
            // "iter-over-list" => context.primitive_iter_over_list(arg),
            // "let" => context.primitive_let(arg),
            "get-key" => context.primitive_get_key(),
            "print" => context.primitive_print(fiber),
            "run-and-import" => context.primitive_run_and_import(fiber),
            "wait" => context.primitive_wait(),
            // "wrap-in-code" => context.primitive_wrap_in_code(fiber),
            // "type" => context.primitive_type(arg),
            _ => panic!("Unknown primitive {}.", name),
        }
    }
    fn primitive_export(mut self) -> Self {
        let args = self.dot.clone().as_map().expect("export expects a map.");
        let funs_to_export = args
            .keys()
            .map(|name| {
                name.clone()
                    .as_symbol()
                    .expect("export expects a map of symbols.")
            })
            .collect::<Vec<_>>();
        println!("Exporting funs {:?}.", funs_to_export);
        for name in funs_to_export {
            let fun = self.funs.get(&name).expect(&format!(
                "Tried to export fun {}, but that doesn't exist.",
                name
            ));
            self.exported_funs.insert(name, fun.clone());
        }
        self
    }
    fn primitive_fun(mut self) -> Result<Self, String> {
        let args = self
            .dot
            .clone()
            .as_map()
            .ok_or(format!("fun expects a map, got: {}", self.dot))?;
        let name = args
            .get_symbol("name")
            .ok_or("fun needs a :name.".to_string())?
            .clone()
            .as_symbol()
            .ok_or("The fun :name needs to be a symbol.".to_string())?;
        let docs = args
            .get_symbol("docs")
            .and_then(|docs| docs.clone().as_string());
        let (scope, body) = args
            .get_symbol("body")
            .ok_or("fun needs a :body.".to_string())?
            .clone()
            .as_code()
            .ok_or("The fun :body needs to be code.".to_string())?;

        let fun = Fun {
            name: name.clone(),
            docs,
            scope: *scope,
            body,
        };
        self.dot = Expr::unit();
        self.funs.insert(name.clone(), Rc::new(fun));
        println!(
            "Defined function {:?}. Known funs: {:?}",
            &name,
            self.funs.keys().collect::<Vec<_>>()
        );
        Ok(self)
    }
    fn primitive_fun_and_export(self, fiber: &mut Fiber) -> Result<Self, String> {
        let name = self
            .dot
            .clone()
            .as_map()
            .unwrap()
            .get(&Expr::Symbol("name".into()))
            .unwrap()
            .clone();
        let map = {
            let mut map = HashMap::new();
            map.insert(name, Expr::unit());
            map
        };
        let context = self
            .primitive_fun()?
            .next(fiber, Expr::Map(map))
            .primitive_export();
        Ok(context)
    }
    fn primitive_get_key(mut self) -> Self {
        let tuple = self.dot.as_list().expect("get-key needs list.");
        let map = tuple[0]
            .clone()
            .as_map()
            .expect("get-key expected map as first argument.");
        let key = tuple[1].clone();
        self.dot = map.get(&key).expect("key not found.").clone();
        self
    }
    fn primitive_loop(self, fiber: &mut Fiber) -> Self {
        let (scope, body) = self.dot.as_code().expect("loop needs code.");
        let context = scope.next(fiber, Expr::unit());
        loop {
            context.clone().run(fiber, body.clone());
        }
    }
    fn primitive_print(self, fiber: &mut Fiber) -> Self {
        println!("🌮> {}", self.dot);
        self.next(fiber, Expr::unit())
    }
    fn primitive_run_and_import(mut self, fiber: &mut Fiber) -> Self {
        let (scope, body) = self
            .dot
            .clone()
            .as_code()
            .expect("run-and-import needs code");
        let context = scope.next(fiber, Expr::unit());
        let result = context.run(fiber, body);
        for (name, fun) in result.exported_funs {
            self.funs.insert(name, fun);
        }
        self
    }
    fn primitive_wait(self) -> Self {
        let seconds = self.dot.clone().as_number().expect("wait needs a number,");
        std::thread::sleep(std::time::Duration::new(seconds as u64, 0));
        self
    }
    // fn primitive_type(&self) -> Self {
    //     let value = match self.dot {
    //         Ast::Number(_) => "number",
    //         Ast::String(_) => "string",
    //         Ast::Symbol(_) => "symbol",
    //         Ast::List(_) => "list",
    //         Ast::Map(_) => "map",
    //         Ast::Code(_) => "code",
    //         Ast::Name(_) => {
    //             panic!("Called primitive type on Name, but it should be executed to an expression.")
    //         }
    //     };
    //     self.with_dot(Ast::Symbol(value.into()))
    // }
}

impl Into<Ast> for bool {
    fn into(self) -> Ast {
        let text = if self { "true" } else { "false" };
        Ast::Symbol(text.into())
    }
}

trait ExprMap {
    fn get_symbol(&self, symbol: &str) -> Option<Expr>;
}
impl ExprMap for HashMap<Expr, Expr> {
    fn get_symbol(&self, symbol: &str) -> Option<Expr> {
        self.get(&Expr::Symbol(symbol.into()))
            .map(|expr| expr.clone())
    }
}
