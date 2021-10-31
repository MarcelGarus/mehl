use super::ir::*;
use crate::ast::{Ast, Asts};
use std::collections::HashMap;

pub fn compile(asts: Asts) -> Ir {
    let mut compiler = Compiler {
        statements: Statements::new(0),
        funs: HashMap::new(),
    };
    let dot = compiler.push(Statement::Symbol("".into()));
    let dot = compiler.compile(dot, asts);
    Ir {
        statements: compiler.statements,
        out: dot,
    }
}

struct Compiler {
    statements: Statements,
    funs: HashMap<String, Id>,
}
impl Compiler {
    fn push(&mut self, action: Statement) -> Id {
        self.statements.push(action)
    }
}

impl Compiler {
    fn compile(&mut self, dot: Id, asts: Asts) -> Id {
        let mut dot = dot;
        for ast in asts {
            dot = self.compile_single(dot, ast);
        }
        dot
    }
    fn compile_single(&mut self, dot: Id, ast: Ast) -> Id {
        match ast {
            Ast::Int(int) => self.push(Statement::Int(int)),
            Ast::String(string) => self.push(Statement::String(string)),
            Ast::Symbol(symbol) => self.push(Statement::Symbol(symbol)),
            Ast::Map(ast_map) => {
                let mut map = HashMap::new();
                for (key, value) in ast_map {
                    let key = self.compile(dot, key);
                    let value = self.compile(dot, value);
                    map.insert(key, value);
                }
                self.push(Statement::Map(map))
            }
            Ast::List(ast_list) => {
                let mut list = vec![];
                for item in ast_list {
                    list.push(self.compile(dot, item));
                }
                self.push(Statement::List(list))
            }
            Ast::Code(code) => {
                let mut inner = Compiler {
                    statements: self.statements.child(),
                    funs: self.funs.clone(),
                };
                let out = inner.compile(self.statements.next_id(), code);
                self.push(Statement::Code {
                    out,
                    statements: inner.statements,
                })
            }
            Ast::Name(name) => match name.as_str() {
                "." => dot,
                "✨" => self.push(Statement::Primitive { arg: dot }),
                name => self.push(Statement::Call {
                    fun: *self
                        .funs
                        .get(name)
                        .expect(&format!("name \"{}\" not found", name)),
                    arg: dot,
                }),
            },
            Ast::Let(name) => {
                let dot = self.push(Statement::Code {
                    out: dot,
                    statements: self.statements.child(),
                });
                self.funs.insert(name, dot);
                self.push(Statement::Symbol("".into()))
            }
            Ast::Fun(name) => {
                self.funs.insert(name, dot);
                self.push(Statement::Symbol("".into()))
            }
        }
    }
}
