use crate::expr::*;
use std::collections::HashMap;

pub fn run(asts: Vec<Ast>) -> Expr {
    Context::new().run_all(asts).dot
}

#[derive(Clone)]
struct Context {
    funs: Vec<Fun>,
    dot: Expr,
}
impl Context {
    fn new() -> Self {
        Self {
            funs: vec![],
            dot: Expr::unit(),
        }
    }
}
#[derive(Clone)]
struct Fun {
    name: String,
    checker: Vec<Ast>,
    body: Vec<Ast>,
}

impl Context {
    pub fn run_all(&self, asts: Vec<Ast>) -> Context {
        let mut context = self.clone();
        for ast in asts {
            context = context.run(ast);
        }
        context
    }

    pub fn run(&self, ast: Ast) -> Context {
        println!("Dot: {}", self.dot);
        match ast {
            Ast::Number(number) => self.with_dot(Expr::Number(number)),
            Ast::String(string) => self.with_dot(Expr::String(string)),
            Ast::Symbol(symbol) => self.with_dot(Expr::Symbol(symbol)),
            Ast::Name(name) => {
                if &name == "." {
                    self.clone()
                } else {
                    self.handle_name(&name)
                }
            }
            Ast::List(list) => {
                let mut expr_list = vec![];
                for item in list {
                    expr_list.push(self.run_all(item).dot);
                }
                self.with_dot(Expr::List(expr_list))
            }
            Ast::Map(map) => {
                let mut expr_map = HashMap::new();
                for (key, value) in map {
                    expr_map.insert(key, self.run(value).dot);
                }
                self.with_dot(Expr::Map(expr_map))
            }
            Ast::Code(list) => self.with_dot(Expr::Code(list)),
        }
    }

    fn with_dot(&self, dot: Expr) -> Context {
        Self {
            dot,
            ..self.clone()
        }
    }

    fn handle_name(&self, name: &str) -> Context {
        match name {
            "primitive" => self.primitive(),
            name => {
                let matching_funs = self
                    .funs
                    .iter()
                    .filter(|it| it.name == name)
                    .collect::<Vec<_>>();
                if matching_funs.len() != 1 {
                    panic!("Unknown name {}", name);
                }
                let fun = matching_funs.iter().nth(0).unwrap();
                let code = fun.body.clone();
                self.run_all(code)
            }
        }
        // let expression = self.bindings.get(name).unwrap();
    }

    fn primitive(&self) -> Context {
        let args = match self.dot.clone() {
            Expr::List(args) => args,
            _ => panic!("Called primitive, but the dot is not a List: {}", self.dot),
        };
        if args.len() != 2 {
            panic!(
                "Called primitive, but the dot doesn't contain exactly two items: {}",
                self.dot
            );
        }
        let name = match &args[0] {
            Expr::Symbol(name) => name,
            _ => panic!(
                "Called primitive, but the dot's first item is not a symbol: {}",
                self.dot
            ),
        };
        let context = self.with_dot(args[1].clone());
        match name.as_ref() {
            "fun" => context.primitive_fun(),
            // "identical" => context.primitive_identical(),
            // "let" => context.primitive_let(),
            "print" => context.primitive_print(),
            "type" => context.primitive_type(),
            _ => panic!("Unknown primitive {}.", name),
        }
    }
    // fn primitive_identical(&mut self) {
    //     let is_identical = self.pop() == self.pop();
    //     self.push(is_identical.into())
    // }
    fn primitive_fun(&self) -> Context {
        let args = match self.dot.clone() {
            Expr::List(args) => args,
            _ => panic!("Called fun, but the dot is not a List: {}", self.dot),
        };
        if args.len() != 3 {
            panic!(
                "Called fun, but the dot doesn't contain exactly two items: {}",
                self.dot
            );
        }
        let name = match args[0].clone() {
            Expr::Symbol(name) => name,
            _ => panic!(
                "Called fun, but the dot's first item is not a symbol: {}",
                self.dot
            ),
        };
        let checker = match args[1].clone() {
            Expr::Code(code) => code,
            _ => panic!(
                "Called fun, but the dot's second item is not a code: {}",
                self.dot
            ),
        };
        let body = match args[2].clone() {
            Expr::Code(code) => code,
            _ => panic!(
                "Called fun, but the dot's third item is not code: {}",
                self.dot
            ),
        };

        println!("Defined function {}", name);
        Context {
            funs: {
                let mut funs = self.funs.clone();
                funs.push(Fun {
                    name,
                    checker,
                    body,
                });
                funs
            },
            ..self.clone()
        }
    }
    fn primitive_print(&self) -> Context {
        println!("🌮> {}", self.dot);
        self.with_dot(Expr::unit())
    }
    fn primitive_type(&self) -> Context {
        let value = match self.dot {
            Expr::Number(_) => "number",
            Expr::String(_) => "string",
            Expr::Symbol(_) => "symbol",
            Expr::List(_) => "list",
            Expr::Map(_) => "map",
            Expr::Code(_) => "code",
        };
        self.with_dot(Expr::Symbol(value.into()))
    }
    // fn primitive_let(&mut self) {
    //     match (self.pop(), self.pop()) {
    //         (Expr::Symbol(name), value) => {
    //             self.lets.insert(name, value);
    //             println!("Definitions are now: {:?}", self.lets);
    //         }
    //         (arg1, arg2) => panic!("Bad operands for define: {} {}", arg1, arg2),
    //     }
    // }
    // fn primitive_print(&mut self) {
    //     println!("Printed> {}", self.pop());
    // }
    // fn primitive_add(&mut self) {
    //     match (self.pop(), self.pop()) {
    //         (Expr::Number(a), Expr::Number(b)) => self.push(Expr::Number(a + b)),
    //         _ => panic!("Bad operands for +"),
    //     }
    // }
}

impl Into<Expr> for bool {
    fn into(self) -> Expr {
        let text = if self { "true" } else { "false" };
        Expr::Symbol(text.into())
    }
}
