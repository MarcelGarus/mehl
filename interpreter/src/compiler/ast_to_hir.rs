use super::ast::{Ast, Asts};
use super::hir::*;
use std::collections::HashMap;

pub trait CompileAstsToHir {
    fn compile_to_hir(self) -> Code;
}
impl CompileAstsToHir for Asts {
    fn compile_to_hir(self) -> Code {
        let mut compiler = Compiler {
            code: Code::new(0, 0),
            funs: HashMap::new(),
        };
        let dot = compiler.push(Statement::unit());
        compiler.compile(dot, self);
        compiler.code
    }
}

struct Compiler {
    code: Code,
    funs: HashMap<String, Id>,
}
impl Compiler {
    fn push(&mut self, action: Statement) -> Id {
        self.code.push(action)
    }
}

impl Compiler {
    fn compile(&mut self, dot: Id, asts: Asts) -> Id {
        let mut dot = dot;
        for ast in asts.into_iter() {
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
                    code: self.code.child_identity(),
                    funs: self.funs.clone(),
                };
                inner.compile(inner.code.in_, code);
                self.push(Statement::Code(inner.code))
            }
            Ast::Name(name) => match name.as_str() {
                "." => dot,
                "✨" => self.push(Statement::Primitive {
                    kind: None,
                    arg: dot,
                }),
                name => self.push(Statement::Call {
                    fun: *self
                        .funs
                        .get(name)
                        .expect(&format!("name \"{}\" not found", name)),
                    arg: dot,
                }),
            },
            Ast::Let(name) => {
                let code = self.push(Statement::Code(self.code.child(dot)));
                self.funs.insert(name, code);
                self.push(Statement::unit())
            }
            Ast::Fun(name) => {
                self.funs.insert(name, dot);
                self.push(Statement::unit())
            }
        }
    }
}
