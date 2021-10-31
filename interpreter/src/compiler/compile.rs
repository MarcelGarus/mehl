use super::ir::*;
use crate::ast::{Ast, Asts};
use std::collections::HashMap;

pub fn compile(asts: Asts) -> Ir {
    let mut compiler = Compiler {
        next_id: 0,
        statements: Statements::new(),
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
    next_id: Id,
    statements: Statements,
    funs: HashMap<String, Id>,
}
impl Compiler {
    fn push(&mut self, action: Statement) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        self.statements.push(id, action);
        id
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
                    next_id: self.next_id + 1,
                    statements: Statements::new(),
                    funs: self.funs.clone(),
                };
                let out = inner.compile(self.next_id, code);
                self.push(Statement::Code {
                    in_: self.next_id,
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
                    in_: self.next_id,
                    out: dot,
                    statements: Statements::new(),
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
