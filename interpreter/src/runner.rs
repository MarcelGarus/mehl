use crate::ast::*;
use crate::expr::*;
use std::{collections::HashMap, rc::Rc};

#[derive(Clone)]
pub struct Context(Rc<ContextData>);
pub struct ContextData {
    previous: Option<Box<Context>>,
    fun: Option<Fun>,
    pub dot: Expr,
}
#[derive(Clone)]
struct Fun {
    name: String,
    docs: Option<String>,
    scope: Box<Context>,
    body: Asts,
}

impl Context {
    pub fn new() -> Self {
        Self(Rc::new(ContextData {
            previous: None,
            fun: None,
            dot: Expr::unit(),
        }))
    }
}

impl Context {
    pub fn run(&self, code: Asts) -> Self {
        let mut context = self.clone();
        for ast in code {
            context = context.run_single(ast);
        }
        context
    }

    fn run_single(&self, ast: Ast) -> Self {
        println!(
            "Running {} on {}",
            format_code(&vec![ast.clone()]),
            self.dot()
        );
        match ast {
            Ast::Number(number) => self.next(None, Expr::Number(number)),
            Ast::String(string) => self.next(None, Expr::String(string)),
            Ast::Symbol(symbol) => self.next(None, Expr::Symbol(symbol)),
            Ast::Map(map) => {
                let mut expr_map = HashMap::new();
                for (key, value) in map {
                    expr_map.insert(self.run(key).0.dot.clone(), self.run(value).0.dot.clone());
                }
                self.next(None, Expr::Map(expr_map))
            }
            Ast::List(list) => {
                let mut expr_list = vec![];
                for item in list {
                    expr_list.push(self.run(item).0.dot.clone());
                }
                self.next(None, Expr::List(expr_list))
            }
            Ast::Code(asts) => self.next(None, Expr::Code(asts)),
            Ast::Name(name) => {
                if &name == "." {
                    self.clone()
                } else {
                    self.handle_name(&name)
                }
            }
        }
    }

    pub fn dot(&self) -> Expr {
        self.0.dot.clone()
    }

    fn resolve(&self, name: &str) -> Option<Fun> {
        match &self.0.fun {
            Some(fun) if fun.name == name => Some(fun.clone()),
            _ => match &self.0.previous {
                Some(previous) => previous.resolve(name),
                None => None,
            },
        }
    }

    fn next(&self, fun: Option<Fun>, dot: Expr) -> Self {
        Self(Rc::new(ContextData {
            previous: Some(Box::new(self.clone())),
            fun,
            dot,
        }))
    }

    fn handle_name(&self, name: &str) -> Self {
        match name {
            "✨" => self.primitive(),
            name => match self.resolve(name) {
                Some(fun) => self.run(fun.body),
                None => panic!("Unknown name {}.", name),
            },
        }
    }

    fn primitive(&self) -> Self {
        let dot = self.dot();
        let args = match self.dot() {
            Expr::List(args) => args,
            _ => panic!("✨ needs a list, got this: {}", self.dot()),
        };
        if args.len() != 2 {
            panic!("✨ needs a list with two items, got this: {}", self.dot());
        }
        let name = match &args[0] {
            Expr::Symbol(name) => name,
            _ => panic!(
                "✨ needs a symbol as the first tuple item, got this: {}",
                dot
            ),
        };
        let arg = args[1].clone();
        match name.as_ref() {
            "fun" => match self.primitive_fun(&arg) {
                Ok(context) => context,
                Err(err) => panic!("{}\nDot: {}", err, arg),
            },
            // "List[Int].+" => self.primitive_add_list_of_ints(arg),
            // "identical" => self.primitive_identical(arg),
            // "iter-over-list" => self.primitive_iter_over_list(arg),
            // "let" => self.primitive_let(arg),
            "print" => self.primitive_print(arg),
            // "type" => self.primitive_type(arg),
            _ => panic!("Unknown primitive {}.", name),
        }
    }
    fn primitive_fun(&self, arg: &Expr) -> Result<Self, String> {
        let args = arg
            .clone()
            .as_map()
            .ok_or(format!("fun expects a map, got: {}", arg))?;
        let name = args
            .get_symbol("name")
            .ok_or("fun needs a :name.".to_string())?
            .clone()
            .as_symbol()
            .ok_or("The fun :name needs to be a symbol.".to_string())?;
        let docs = args
            .get_symbol("docs")
            .and_then(|docs| docs.clone().as_string());
        let body = args
            .get_symbol("body")
            .ok_or("fun needs a :body.".to_string())?
            .clone()
            .as_code()
            .ok_or("The fun :body needs to be code.".to_string())?;

        let fun = Fun {
            name,
            docs,
            scope: Box::new(self.clone()),
            body,
        };
        println!("Defined function {:?}.", &fun.name);
        Ok(self.next(Some(fun), Expr::unit()))
    }
    // fn primitive_iter_over_list(&self) -> Self {
    //     let list = match self.dot.clone() {
    //         Ast::List(list) => list,
    //         _ => panic!("Bad input: {}", self.dot),
    //     };
    //     self.with_dot(Ast::List(vec![
    //         list.first().expect("Bad input.").clone(),
    //         Ast::List(list.into_iter().skip(1).collect()),
    //     ]))
    // }
    fn primitive_print(&self, arg: Expr) -> Self {
        println!("🌮> {}", arg);
        self.next(None, Expr::unit())
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
