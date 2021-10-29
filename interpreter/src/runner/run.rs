use colored::*;
use im::HashMap;
use itertools::Itertools;
use std::rc::Rc;

use super::{runtime::*, utils::*};
use crate::ast::*;

type RunResult = Result<Context, Expr>;
fn error(kind: String, msg: String) -> Expr {
    Expr::List(vec![Expr::Symbol(kind), Expr::String(msg)])
}
fn wrong_usage(msg: String) -> Expr {
    error("wrong-usage".into(), msg)
}
fn unknown_function(msg: String) -> Expr {
    error("unknown-fun".into(), msg)
}

impl Context {
    pub fn run(self, runtime: &mut Runtime, code: Asts) -> RunResult {
        let mut context = self.clone();
        for ast in code {
            context = context.run_single(runtime, ast)?;
        }
        Ok(context)
    }

    fn run_single(self, runtime: &mut Runtime, ast: Ast) -> RunResult {
        runtime.log(&format!(
            "Running {} on {}. Funs: {}",
            format_code(&vec![ast.clone()]).yellow(),
            self.dot.to_string().green(),
            self.funs.to_fancy_string(),
        ));
        Ok(match ast {
            Ast::Number(number) => self.next(runtime, Expr::Number(number)),
            Ast::String(string) => self.next(runtime, Expr::String(string)),
            Ast::Symbol(symbol) => self.next(runtime, Expr::Symbol(symbol)),
            Ast::Map(map) => {
                let mut expr_map = HashMap::new();
                let context = self.clone();
                runtime.depth_increase();
                for (key, value) in map {
                    expr_map.insert(
                        context.clone().run(runtime, key)?.dot,
                        context.clone().run(runtime, value)?.dot,
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
                    expr_list.push(context.clone().run(runtime, item)?.dot);
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
                    return Ok(self.clone());
                }
                let fun = self.funs.get(&name).ok_or(unknown_function(name.clone()))?;
                runtime.depth_increase();
                let context = match fun.body.clone() {
                    FunBody::Primitive => self.clone().primitive(runtime)?,
                    FunBody::Code { scope, body } => (*scope)
                        .clone()
                        .next(runtime, self.dot.clone())
                        .run(runtime, body.to_vec())?,
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
        })
    }
}

// Primitives.
impl Context {
    fn primitive(self, runtime: &mut Runtime) -> RunResult {
        let args = self
            .dot
            .clone()
            .as_list()
            .ok_or(wrong_usage("✨ needs a list.".into()))?;
        if args.len() != 2 {
            return Err(wrong_usage("✨ needs a list with two items.".into()));
        }
        let name = args[0].clone().as_symbol().ok_or(wrong_usage(
            "✨ needs a symbol as the first tuple item".into(),
        ))?;
        let arg = args[1].clone();
        let context = self.clone().next(runtime, arg.clone());
        match name.as_ref() {
            "export-all" => Ok(context.primitive_export_all()),
            "fun" => context.primitive_fun(runtime),
            "let" => context.primitive_let(runtime),
            "get-item" => context.primitive_get_item(),
            "get-key" => context.primitive_get_key(),
            "loop" => context.primitive_loop(runtime),
            "match" => context.primitive_match(runtime),
            "panic" => context.primitive_panic(),
            "print" => Ok(context.primitive_print(runtime)),
            "use" => context.primitive_use(runtime),
            "wait" => context.primitive_wait(runtime),
            _ => Err(wrong_usage(format!("Unknown primitive {}.", name))),
        }
    }

    fn primitive_export_all(mut self) -> Self {
        self.funs = self
            .funs
            .into_iter()
            .map(|(name, mut fun)| {
                fun.export_level += 2;
                (name, fun)
            })
            .collect();
        self.dot = Expr::unit();
        self
    }

    fn primitive_fun(mut self, runtime: &mut Runtime) -> RunResult {
        let args = self
            .dot
            .clone()
            .as_map()
            .ok_or(wrong_usage("fun needs a map.".into()))?;
        let name = args
            .get_symbol("name")
            .ok_or(wrong_usage("fun needs a :name.".into()))?
            .clone()
            .as_symbol()
            .ok_or(wrong_usage("fun :name needs to be a symbol.".into()))?;
        let export_level = args
            .get_symbol("export-level")
            .unwrap_or(Expr::Number(0))
            .as_number()
            .ok_or(wrong_usage(
                "fun :export-level needs to be a number.".into(),
            ))? as u16
            + 1;
        let docs = args
            .get_symbol("docs")
            .and_then(|docs| docs.clone().as_string());
        let (scope, body) = args
            .get_symbol("body")
            .ok_or(wrong_usage("fun needs a :body.".into()))?
            .clone()
            .as_code()
            .ok_or(wrong_usage("fun :body needs to be code.".into()))?;

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

    fn primitive_get_item(mut self) -> RunResult {
        let tuple = self
            .dot
            .as_list()
            .ok_or(wrong_usage("get-item needs a list.".into()))?;
        let list = tuple[0].clone().as_list().ok_or(wrong_usage(
            "get-item needs a list as the first argument.".into(),
        ))?;
        let index = tuple[1].clone().as_number().unwrap();
        self.dot = list[index as usize].clone();
        Ok(self)
    }

    fn primitive_get_key(mut self) -> RunResult {
        let tuple = self
            .dot
            .as_list()
            .ok_or(wrong_usage("get-key needs list.".into()))?;
        let map = tuple[0].clone().as_map().ok_or(wrong_usage(
            "get-key needs a map as the first argument.".into(),
        ))?;
        let key = tuple[1].clone();
        // TODO: Return Maybe.
        self.dot = map.get(&key).expect("key not found.").clone();
        Ok(self)
    }

    fn primitive_let(mut self, runtime: &mut Runtime) -> RunResult {
        let args = self
            .dot
            .clone()
            .as_map()
            .ok_or(wrong_usage("let needs a map.".into()))?;
        let name = args
            .get_symbol("name")
            .ok_or(wrong_usage("let needs a :name.".into()))?
            .clone();
        let export_level = args
            .get_symbol("export-level")
            .unwrap_or(Expr::Number(0))
            .as_number()
            .ok_or(wrong_usage(
                "let :export-level needs to be a number.".into(),
            ))? as u16
            + 1;
        let docs = args
            .get_symbol("docs")
            .and_then(|docs| docs.clone().as_string());
        let value = args
            .get_symbol("value")
            .ok_or(wrong_usage("let needs a :value.".into()))?;

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
        // TODO: This still panics.
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

    fn primitive_loop(self, runtime: &mut Runtime) -> RunResult {
        let (scope, body) = self
            .dot
            .as_code()
            .ok_or(wrong_usage("loop needs code.".into()))?;
        let context = scope.next(runtime, Expr::unit());
        loop {
            context.clone().run(runtime, body.clone())?;
        }
    }

    fn primitive_match(self, runtime: &mut Runtime) -> RunResult {
        let list = self
            .dot
            .as_list()
            .ok_or(wrong_usage("match needs a list.".into()))?;
        {
            // Usage checks.
            if list.len() < 3 {
                return Err(wrong_usage(
                        "match needs a list with at least 3 items – the value, a pattern, and some code."
                            .into(),
                    ));
            }
            if list.len() % 2 == 0 {
                return Err(wrong_usage("match needs a list with an odd number of items – the value, and then pairs of patterns and code.".into()));
            }
            let mut i = 2;
            while i < list.len() {
                list[i].clone().as_code().ok_or(wrong_usage(
                    "match needs a value, and then in turn conditions and code.".into(),
                ))?;
                i += 2;
            }
        }

        let value = list[0].clone();
        for mut chunk in &list.into_iter().skip(1).chunks(2) {
            let (condition, code) = (chunk.next().unwrap(), chunk.next().unwrap());
            let bindings = match Self::match_helper(&condition, &value) {
                Some(bindings) => bindings,
                None => continue,
            };
            let (scope, body) = code.as_code().expect("checked above");
            let mut context = scope.next(runtime, Expr::unit());
            for (key, value) in bindings {
                context.funs.insert(
                    key.clone(),
                    Fun {
                        name: key,
                        docs: None,
                        body: FunBody::Value(Rc::new(value)),
                        export_level: 0,
                    },
                );
            }
            return context.run(runtime, body.clone());
        }
        Err(wrong_usage("no condition matched".into()))
    }
    fn match_helper(left: &Expr, right: &Expr) -> Option<HashMap<String, Expr>> {
        fn literal_match<T: Eq>(left: &T, right: &T) -> Option<HashMap<String, Expr>> {
            if left == right {
                Some(HashMap::new())
            } else {
                None
            }
        }
        match left {
            Expr::Number(_) => literal_match(left, right),
            Expr::String(_) => literal_match(left, right),
            Expr::Symbol(symbol) => {
                if symbol.starts_with("?") {
                    let mut map = HashMap::new();
                    map.insert(symbol[1..].to_string(), right.clone());
                    Some(map)
                } else {
                    literal_match(left, right)
                }
            }
            Expr::Map(left_map) => {
                let mut unified = HashMap::new();
                let right_map = right.clone().as_map()?;
                for (key, left_value) in left_map {
                    let bindings = Self::match_helper(left_value, right_map.get(&key)?)?;
                    for (name, value) in bindings {
                        if let Some(expected_value) = unified.get(&name) {
                            literal_match(&value, expected_value)?;
                        } else {
                            unified.insert(name, value);
                        }
                    }
                }
                Some(unified)
            }
            Expr::List(left_list) => {
                let mut unified = HashMap::new();
                let right_list = right.clone().as_list()?;
                if left_list.len() != right_list.len() {
                    return None;
                }
                for (left, right) in left_list.into_iter().zip(right_list.iter()) {
                    let bindings = Self::match_helper(left, right)?;
                    for (name, value) in bindings {
                        if let Some(expected_value) = unified.get(&name) {
                            literal_match(&value, expected_value)?;
                        } else {
                            unified.insert(name, value);
                        }
                    }
                }
                Some(unified)
            }
            Expr::Code { scope: _, asts: _ } => literal_match(left, right),
        }
    }

    fn primitive_panic(self) -> RunResult {
        Err(self.dot)
    }

    fn primitive_print(self, runtime: &mut Runtime) -> Self {
        runtime.print(&self.dot);
        self.next(runtime, Expr::unit())
    }

    fn primitive_use(mut self, runtime: &mut Runtime) -> RunResult {
        let (scope, body) = self
            .dot
            .clone()
            .as_code()
            .ok_or(wrong_usage("use needs code".into()))?;
        let context = scope.next(runtime, Expr::unit());
        let result = context.run(runtime, body)?;
        for (name, fun) in result.funs {
            self.funs.insert(name, fun);
        }
        Ok(self)
    }

    fn primitive_wait(self, runtime: &mut Runtime) -> RunResult {
        let seconds = self
            .dot
            .clone()
            .as_number()
            .ok_or(wrong_usage("wait needs a number.".into()))?;
        if seconds < 0 {
            return Err(wrong_usage(
                "can't wait a negative number of seconds.".into(),
            ));
        }
        runtime.wait(seconds as u64);
        Ok(self)
    }
}
