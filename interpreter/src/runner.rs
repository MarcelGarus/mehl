use crate::ast::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct RunContext {
    funs: Vec<Fun>,
    pub dot: Ast,
}
impl RunContext {
    pub fn new() -> Self {
        Self {
            funs: vec![],
            dot: Ast::unit(),
        }
    }
}
#[derive(Debug, Clone)]
struct Fun {
    name: String,
    type_in: Vec<Ast>,
    type_out: Vec<Ast>,
    docs: Option<String>,
    panics_if: Option<String>,
    body: Vec<Ast>,
}

impl RunContext {
    pub fn run_all(&self, code: Vec<Ast>) -> Self {
        let mut context = self.clone();
        for ast in code {
            context = context.run(ast);
        }
        context
    }

    pub fn run(&self, ast: Ast) -> Self {
        println!("Dot: {}", self.dot);
        match ast {
            Ast::Number(_) | Ast::String(_) | Ast::Symbol(_) | Ast::Code(_) => self.with_dot(ast),
            Ast::Map(map) => {
                let mut expr_map = HashMap::new();
                for (key, value) in map {
                    expr_map.insert(key, self.run(value).dot);
                }
                self.with_dot(Ast::Map(expr_map))
            }
            Ast::List(list) => {
                let mut expr_list = vec![];
                for item in list {
                    expr_list.push(self.run(item).dot);
                }
                self.with_dot(Ast::List(expr_list))
            }
            Ast::Name(name) => {
                if &name == "." {
                    self.clone()
                } else {
                    self.handle_name(&name)
                }
            }
        }
    }

    fn with_dot(&self, dot: Ast) -> Self {
        Self {
            dot,
            ..self.clone()
        }
    }

    fn handle_name(&self, name: &str) -> Self {
        match name {
            "magic-primitive" => self.primitive(),
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

    fn primitive(&self) -> Self {
        let args = match self.dot.clone() {
            Ast::List(args) => args,
            _ => panic!("Called primitive, but the dot is not a list: {}", self.dot),
        };
        if args.len() != 2 {
            panic!(
                "Called primitive, but the dot doesn't contain exactly two items: {}",
                self.dot
            );
        }
        let name = match &args[0] {
            Ast::Symbol(name) => name,
            _ => panic!(
                "Called primitive, but the dot's first item is not a symbol: {}",
                self.dot
            ),
        };
        let context = self.with_dot(args[1].clone());
        match name.as_ref() {
            "List[Int].+" => context.primitive_add_list_of_ints(),
            "fun" => match context.primitive_fun() {
                Ok(context) => context,
                Err(err) => panic!("{}\nDot: {}", err, context.dot),
            },
            "identical" => context.primitive_identical(),
            "iter-over-list" => context.primitive_iter_over_list(),
            // "let" => context.primitive_let(),
            "print" => context.primitive_print(),
            "type" => context.primitive_type(),
            _ => panic!("Unknown primitive {}.", name),
        }
    }
    fn primitive_add_list_of_ints(&self) -> Self {
        let args = match self.dot.clone() {
            Ast::List(args) => args,
            _ => panic!("Called fun, but the dot is not a list: {}", self.dot),
        };
        let sum = args
            .iter()
            .map(|arg| match arg {
                Ast::Number(n) => n,
                _ => panic!("Called add-numbers, but the list contains stuff other than numbers."),
            })
            .sum();
        self.with_dot(Ast::Number(sum))
    }
    fn primitive_fun(&self) -> Result<Self, String> {
        let args = self
            .dot
            .clone()
            .as_map()
            .ok_or("Called fun, but the dot is not a map.".to_string())?;
        let name = args
            .get_symbol("name")
            .ok_or("Called fun, but the map doesn't contain a :name.".to_string())?
            .clone()
            .as_symbol()
            .ok_or("Called fun, but the :name is not a symbol.".to_string())?;
        let type_in = args
            .get_symbol("in")
            .clone()
            .ok_or("Called fun, but no :in given.".to_string())?
            .clone()
            .as_code()
            .ok_or("Called fun, but the :in is not code.")?;
        let type_out = args
            .get_symbol("out")
            .ok_or("Called fun, but no :out given.".to_string())?
            .clone()
            .as_code()
            .ok_or("Called fun, but the :out is not code.")?;
        let docs = args
            .get_symbol("docs")
            .and_then(|docs| docs.clone().as_string());
        let panics_if = args
            .get_symbol("panics-if")
            .and_then(|panics_if| panics_if.clone().as_string());
        let body = args
            .get_symbol("body")
            .ok_or("Called fun, but no :body given.".to_string())?
            .clone()
            .as_code()
            .ok_or("Called fun, but the :body is not code.".to_string())?;

        let fun = Fun {
            name,
            type_in,
            type_out,
            docs,
            panics_if,
            body,
        };
        println!("Defined function: {:?}", &fun);
        Ok(Self {
            funs: {
                let mut funs = self.funs.clone();
                funs.push(fun);
                funs
            },
            ..self.clone()
        })
    }
    fn primitive_identical(&self) -> Self {
        let args = match self.dot.clone() {
            Ast::List(args) => args,
            _ => panic!("Called identical, but the dot is not a list: {}", self.dot),
        };
        if args.len() != 2 {
            panic!(
                "Called identical, but the dot doesn't contain exactly two items: {}",
                self.dot
            );
        }
        let is_identical = args[0] == args[1];
        self.with_dot(is_identical.into())
    }
    fn primitive_iter_over_list(&self) -> Self {
        let list = match self.dot.clone() {
            Ast::List(list) => list,
            _ => panic!("Bad input: {}", self.dot),
        };

        self.with_dot(Ast::List(vec![
            list.first().expect("Bad input.").clone(),
            Ast::List(list.into_iter().skip(1).collect()),
        ]))
    }
    fn primitive_print(&self) -> Self {
        println!("🌮> {}", self.dot);
        self.with_dot(Ast::unit())
    }
    fn primitive_type(&self) -> Self {
        let value = match self.dot {
            Ast::Number(_) => "number",
            Ast::String(_) => "string",
            Ast::Symbol(_) => "symbol",
            Ast::List(_) => "list",
            Ast::Map(_) => "map",
            Ast::Code(_) => "code",
            Ast::Name(_) => {
                panic!("Called primitive type on Name, but it should be executed to an expression.")
            }
        };
        self.with_dot(Ast::Symbol(value.into()))
    }
    // fn primitive_val(&self) -> Self {
    //     let args = match self.dot.clone() {
    //         Ast::List(args) => args,
    //         _ => panic!("Bad input: {}", self.dot),
    //     };
    //     if args.len() != 2 {
    //         panic!("Bad input: {}", self.dot);
    //     }
    //     let name = match args[0].clone() {
    //         Ast::Symbol(name) => name,
    //         _ => panic!("Bad input: {}", self.dot),
    //     };
    //     let value = args[1].clone();

    //     println!("Defined value {}", name);
    //     Self {
    //         funs: {
    //             let mut funs = self.funs.clone();
    //             funs.push(Fun {
    //                 name,
    //                 checker: vec![Ast::Code(vec![true.into()])],
    //                 body: vec![value],
    //             });
    //             funs
    //         },
    //         ..self.clone()
    //     }
    // }
    // fn primitive_let(&mut self) {
    //     match (self.pop(), self.pop()) {
    //         (Ast::Symbol(name), value) => {
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
    //         (Ast::Number(a), Ast::Number(b)) => self.push(Ast::Number(a + b)),
    //         _ => panic!("Bad operands for +"),
    //     }
    // }
}

impl Into<Ast> for bool {
    fn into(self) -> Ast {
        let text = if self { "true" } else { "false" };
        Ast::Symbol(text.into())
    }
}
