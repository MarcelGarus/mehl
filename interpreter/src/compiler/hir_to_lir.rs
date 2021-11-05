use super::hir;
use super::lir::*;
use std::collections::HashSet;

pub fn compile(code: &hir::Code) -> Closure {
    Closure {
        captured: {
            let mut used = HashSet::new();
            for (_, statement) in code.iter() {
                statement.collect_used_ids(&mut used);
            }
            used
        },
        in_: code.in_,
        out: code.out,
        code: {
            let mut statements = code
                .iter()
                .flat_map(|(id, statement)| statement.compile(id))
                .collect::<Vec<_>>();
            let mut ids_to_drop = HashSet::new();
            for statement in &statements {
                if let Statement::Assignment { id, .. } = statement {
                    ids_to_drop.insert(*id);
                }
            }
            ids_to_drop.remove(&code.out);
            for id in ids_to_drop {
                statements.push(Statement::Drop(id));
            }
            statements
        },
    }
}

impl hir::Statement {
    fn compile(&self, id: Id) -> Vec<Statement> {
        match self {
            hir::Statement::Int(int) => vec![Statement::Assignment {
                id,
                value: Expr::Int(*int),
            }],
            hir::Statement::String(string) => vec![Statement::Assignment {
                id,
                value: Expr::String(string.clone()),
            }],
            hir::Statement::Symbol(symbol) => vec![Statement::Assignment {
                id,
                value: Expr::Symbol(symbol.clone()),
            }],
            hir::Statement::Map(map) => {
                let mut statements = vec![];
                for (key, value) in map {
                    statements.push(Statement::Dup(*key));
                    statements.push(Statement::Dup(*value));
                }
                statements.push(Statement::Assignment {
                    id,
                    value: Expr::Map(map.clone()),
                });
                statements
            }
            hir::Statement::List(list) => {
                let mut statements = vec![];
                for item in list {
                    statements.push(Statement::Dup(*item));
                }
                statements.push(Statement::Assignment {
                    id,
                    value: Expr::List(list.clone()),
                });
                statements
            }
            hir::Statement::Code(code) => {
                let closure = compile(code);
                let mut statements = vec![];
                for captured_var in &closure.captured {
                    statements.push(Statement::Dup(*captured_var));
                }
                statements.push(Statement::Assignment {
                    id,
                    value: Expr::Closure(closure),
                });
                statements
            }
            hir::Statement::Call { fun, arg } => {
                let mut statements = vec![];
                statements.push(Statement::Dup(*fun));
                statements.push(Statement::Dup(*arg));
                statements.push(Statement::Assignment {
                    id,
                    value: Expr::Call {
                        closure: *fun,
                        arg: *arg,
                    },
                });
                statements
            }
            hir::Statement::Primitive { kind, arg } => {
                let mut statements = vec![];
                statements.push(Statement::Dup(*arg));
                statements.push(Statement::Assignment {
                    id,
                    value: Expr::Primitive {
                        kind: *kind,
                        arg: *arg,
                    },
                });
                statements
            }
        }
    }
}
