use super::hir;
use super::lir::*;
use std::collections::HashSet;

impl hir::Code {
    pub fn compile_to_lir(&self) -> Closure {
        Closure {
            captured: {
                let mut used = HashSet::new();
                for (_, statement) in self.iter() {
                    statement.collect_used_ids(&mut used);
                }
                used.insert(self.out);
                for (id, _) in self.iter() {
                    used.remove(&id);
                }
                let mut used = used.into_iter().collect::<Vec<_>>();
                used.sort();
                used
            },
            in_: self.in_,
            out: self.out,
            code: {
                let mut statements = self
                    .iter()
                    .flat_map(|(id, statement)| statement.compile(id))
                    .collect::<Vec<_>>();
                let mut ids_to_drop = HashSet::new();
                for statement in &statements {
                    if let Statement::Assignment { id, .. } = statement {
                        ids_to_drop.insert(*id);
                    }
                }
                ids_to_drop.insert(self.in_);
                ids_to_drop.remove(&self.out);
                for id in ids_to_drop {
                    statements.push(Statement::Drop(vec![id]));
                }
                statements
            },
        }
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
            hir::Statement::Map(map) => vec![
                Statement::Dup(
                    map.iter()
                        .flat_map(|(k, v)| vec![*k, *v].into_iter())
                        .collect(),
                ),
                Statement::Assignment {
                    id,
                    value: Expr::Map(map.clone()),
                },
            ],
            hir::Statement::List(list) => vec![
                Statement::Dup(list.iter().map(|i| *i).collect()),
                Statement::Assignment {
                    id,
                    value: Expr::List(list.clone()),
                },
            ],
            hir::Statement::Code(code) => {
                let closure = code.compile_to_lir();
                let mut statements = vec![];
                statements.push(Statement::Dup(closure.captured.clone()));
                statements.push(Statement::Assignment {
                    id,
                    value: Expr::Closure(closure),
                });
                statements
            }
            hir::Statement::Call { fun, arg } => vec![
                Statement::Dup(vec![*fun, *arg]),
                Statement::Assignment {
                    id,
                    value: Expr::Call {
                        closure: *fun,
                        arg: *arg,
                    },
                },
            ],
            hir::Statement::Primitive { kind, arg } => vec![
                Statement::Dup(vec![*arg]),
                Statement::Assignment {
                    id,
                    value: Expr::Primitive {
                        kind: *kind,
                        arg: *arg,
                    },
                },
            ],
        }
    }
}
