use crate::ast::*;
use colored::*;
use im::HashMap;
use std::rc::Rc;

use super::runtime::*;

impl Context {
    pub fn run(self, runtime: &mut Runtime, code: Asts) -> Self {
        let mut context = self.clone();
        for ast in code {
            context = context.run_single(runtime, ast);
        }
        context
    }

    fn run_single(self, runtime: &mut Runtime, ast: Ast) -> Self {
        runtime.log(&format!(
            "Running {} on {}. Funs: {}",
            format_code(&vec![ast.clone()]).yellow(),
            self.dot.to_string().green(),
            self.funs.to_fancy_string(),
        ));
        match ast {
            Ast::Number(number) => self.next(runtime, Expr::Number(number)),
            Ast::String(string) => self.next(runtime, Expr::String(string)),
            Ast::Symbol(symbol) => self.next(runtime, Expr::Symbol(symbol)),
            Ast::Map(map) => {
                let mut expr_map = HashMap::new();
                let context = self.clone();
                runtime.depth_increase();
                for (key, value) in map {
                    expr_map.insert(
                        context.clone().run(runtime, key).dot,
                        context.clone().run(runtime, value).dot,
                    );
                }
                runtime.depth_decrease();
                self.next(runtime, Expr::Map(expr_map))
            }
            Ast::List(list) => {
                let mut expr_list = vec![];
                let context = self.clone();
                runtime.depth_increase();
                for item in list {
                    expr_list.push(context.clone().run(runtime, item).dot);
                }
                runtime.depth_decrease();
                self.next(runtime, Expr::List(expr_list))
            }
            Ast::Code(asts) => self.clone().next(
                runtime,
                Expr::Code {
                    scope: Box::new(self),
                    asts,
                },
            ),
            Ast::Name(name) => {
                if name == "." {
                    return self.clone();
                }
                let fun = self
                    .funs
                    .get(&name)
                    .expect(&format!("Unknown name {}.", &name));
                runtime.depth_increase();
                let context = match fun.body.clone() {
                    FunBody::Primitive => self.clone().primitive(runtime),
                    FunBody::Code { scope, body } => (*scope)
                        .clone()
                        .next(runtime, self.dot.clone())
                        .run(runtime, body.to_vec()),
                    FunBody::Value(expr) => self.clone().next(runtime, (*expr).clone()),
                };
                runtime.depth_decrease();
                let mut next_context = self.clone().next(runtime, context.dot);
                for (name, fun) in context.funs.clone() {
                    if fun.export_level >= 1 {
                        let mut fun = fun.clone();
                        fun.export_level -= 1;
                        next_context.funs.insert(name, fun);
                    } else {
                        runtime.log(&format!("Not exporting {}.", name));
                    }
                }
                runtime.log(&format!(
                    "Exited fun {}. Dot: {}, Funs: {}",
                    name.magenta(),
                    self.dot.to_string().green(),
                    self.funs.to_fancy_string(),
                ));
                next_context
            }
        }
    }

    fn primitive(self, runtime: &mut Runtime) -> Self {
        let dot_for_panic_messages = self.dot.clone();
        let args = match self.dot.clone() {
            Expr::List(args) => args,
            _ => panic!("✨ needs a list, got this: {}", dot_for_panic_messages),
        };
        if args.len() != 2 {
            panic!(
                "✨ needs a list with two items, got this: {}",
                dot_for_panic_messages
            );
        }
        let name = match &args[0] {
            Expr::Symbol(name) => name,
            _ => panic!(
                "✨ needs a symbol as the first tuple item, got this: {}",
                dot_for_panic_messages
            ),
        };
        let arg = args[1].clone();
        let context = self.clone().next(runtime, arg.clone());
        match name.as_ref() {
            "fun" => match context.primitive_fun(runtime) {
                Ok(context) => context,
                Err(err) => panic!("{}\nDot: {}", err, arg),
            },
            "let" => match context.primitive_let(runtime) {
                Ok(context) => context,
                Err(err) => panic!("{}\nDot: {}", err, arg),
            },
            "get-item" => context.primitive_get_item(),
            "get-key" => context.primitive_get_key(),
            "loop" => context.primitive_loop(runtime),
            "print" => context.primitive_print(runtime),
            "use" => context.primitive_use(runtime),
            "wait" => context.primitive_wait(runtime),
            _ => panic!("Unknown primitive {}.", name),
        }
    }
    fn primitive_fun(mut self, runtime: &mut Runtime) -> Result<Self, String> {
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
        let export_level = args
            .get_symbol("export-level")
            .unwrap_or(Expr::Number(0))
            .as_number()
            .ok_or("The fun :export-level needs to be a number.".to_string())?
            as u16
            + 1;
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
            body: FunBody::Code {
                scope: Rc::new(*scope),
                body: Rc::new(body),
            },
            export_level,
        };
        self.dot = Expr::unit();
        self.funs.insert(name.clone(), fun);
        runtime.log(&format!(
            "Defined function {:?}. Known funs: {:?}",
            &name,
            self.funs.keys().collect::<Vec<_>>()
        ));
        Ok(self)
    }
    fn primitive_get_item(mut self) -> Self {
        let tuple = self.dot.as_list().expect("get-item needs a list.");
        let list = tuple[0]
            .clone()
            .as_list()
            .expect("get-item expected list as first argument.");
        let index = tuple[1].clone().as_number().unwrap();
        self.dot = list[index as usize].clone();
        self
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
    fn primitive_let(mut self, runtime: &mut Runtime) -> Result<Self, String> {
        let args = self
            .dot
            .clone()
            .as_map()
            .ok_or(format!("let expects a map, got: {}", self.dot))?;
        let name = args
            .get_symbol("name")
            .ok_or("let needs a :name.".to_string())?
            .clone();
        let export_level = args
            .get_symbol("export-level")
            .unwrap_or(Expr::Number(0))
            .as_number()
            .ok_or("The let :export-level needs to be a number.".to_string())?
            as u16
            + 1;
        let docs = args
            .get_symbol("docs")
            .and_then(|docs| docs.clone().as_string());
        let value = args
            .get_symbol("value")
            .ok_or("let needs a :value.".to_string())?;

        let mut definitions = HashMap::new();
        Self::let_helper(&name, &value, &mut definitions);

        for (name, value) in definitions {
            let fun = Fun {
                name: name.clone(),
                docs: docs.clone(),
                body: FunBody::Value(Rc::new(value)),
                export_level,
            };
            self.funs.insert(name.clone(), fun);
            runtime.log(&format!(
                "Defined function {:?}. Known funs: {:?}",
                &name,
                self.funs.keys().collect::<Vec<_>>()
            ));
        }
        self.dot = Expr::unit();
        Ok(self)
    }
    fn let_helper(name: &Expr, value: &Expr, out: &mut HashMap<String, Expr>) {
        match name {
            Expr::Symbol(name) => {
                out.insert(name.clone(), value.clone());
            }
            Expr::Map(name_map) => {
                let value_map = value.clone().as_map().unwrap();
                for (key, name) in name_map {
                    Self::let_helper(name, value_map.get(&key).unwrap(), out);
                }
            }
            Expr::List(name_list) => {
                let value_list = value.clone().as_list().unwrap();
                if name_list.len() != value_list.len() {
                    panic!("List has different length.");
                }
                for (name, value) in name_list.into_iter().zip(value_list.iter()) {
                    Self::let_helper(name, value, out);
                }
            }
            _ => panic!("Invalid match data on left side of let."),
        };
    }
    fn primitive_loop(self, runtime: &mut Runtime) -> Self {
        let (scope, body) = self.dot.as_code().expect("loop needs code.");
        let context = scope.next(runtime, Expr::unit());
        loop {
            context.clone().run(runtime, body.clone());
        }
    }
    fn primitive_print(self, runtime: &mut Runtime) -> Self {
        runtime.print(&self.dot);
        self.next(runtime, Expr::unit())
    }
    fn primitive_use(mut self, runtime: &mut Runtime) -> Self {
        let (scope, body) = self
            .dot
            .clone()
            .as_code()
            .expect("run-and-import needs code");
        let context = scope.next(runtime, Expr::unit());
        let result = context.run(runtime, body);
        for (name, fun) in result.funs {
            self.funs.insert(name, fun);
        }
        self
    }
    fn primitive_wait(self, runtime: &mut Runtime) -> Self {
        let seconds = self.dot.clone().as_number().expect("wait needs a number,");
        runtime.wait(seconds as u64);
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

trait ExprMap {
    fn get_symbol(&self, symbol: &str) -> Option<Expr>;
}
impl ExprMap for HashMap<Expr, Expr> {
    fn get_symbol(&self, symbol: &str) -> Option<Expr> {
        self.get(&Expr::Symbol(symbol.into()))
            .map(|expr| expr.clone())
    }
}

trait FancyFunsExt {
    fn to_fancy_string(&self) -> String;
}
impl FancyFunsExt for HashMap<String, Fun> {
    fn to_fancy_string(&self) -> String {
        itertools::join(
            self.iter().map(|(name, fun)| {
                format!("{}{}", name.blue(), fun.export_level.to_string().red())
            }),
            ", ",
        )
    }
}
