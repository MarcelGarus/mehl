use super::lir::*;
use std::collections::HashSet;

pub trait OptimizeLir {
    fn optimize(&mut self);
}
impl OptimizeLir for Closure {
    fn optimize(&mut self) {
        self.code.optimize();
    }
}
impl OptimizeLir for Vec<Statement> {
    /// Optimizes the LIR. This mostly works by combining dup and drop
    /// statements (they cancel each other out in certain conditions).
    /// The basic idea is to go through the statements from the bottom to the
    /// top and keep track of which ids still need to be dropped.
    fn optimize(&mut self) {
        let mut to_drop = vec![];
        let mut out = vec![];
        let mut statements = self.iter().rev().peekable();
        while let Some(statement) = statements.next() {
            match statement {
                Statement::Drop(ids) => to_drop.append(&mut ids.clone()),
                Statement::Dup(ids) => {
                    // In constellations like these, some dups and drops cancel each other out:
                    // > dup 1, 2, 3
                    // > drop 2, 3
                    // becomes:
                    // > dup 1
                    // > drop <none>
                    let mut ids = ids.clone();
                    cancel_out(&mut to_drop, &mut ids);
                    if !ids.is_empty() {
                        out.push(Statement::Dup(ids));
                    }
                }
                Statement::Assignment { id, value } => {
                    // In constellations like these, some dups and drops cancel each other out:
                    // > dup 1, 2, 3
                    // > xxx
                    // > drop 2, 3
                    // becomes:
                    // > dup 1
                    // xxx
                    // > drop <none>
                    let ids_to_dup = if let Some(Statement::Dup(to_dup)) = statements.peek() {
                        statements.next().unwrap();
                        let mut to_dup = to_dup.clone();
                        cancel_out(&mut to_drop, &mut to_dup);
                        Some(to_dup)
                    } else {
                        None
                    };
                    // Remembered ids to_drop cannot pass this expression if they're used in it.
                    let mut used_ids = vec![];
                    value.collect_used_ids(&mut used_ids);
                    cancel_out(&mut to_drop, &mut used_ids);
                    // A drop we keep track of can't pass the creation statement.
                    if let Some(index) = to_drop.iter().position(|it| it == id) {
                        out.push(Statement::Drop(vec![*id]));
                        to_drop.remove(index);
                    }
                    out.push(Statement::Assignment {
                        id: *id,
                        value: {
                            let value = value.clone();
                            if let Expr::Closure(mut closure) = value {
                                closure.optimize();
                                Expr::Closure(closure)
                            } else {
                                value
                            }
                        },
                    });

                    if let Some(ids) = ids_to_dup {
                        if !ids.is_empty() {
                            out.push(Statement::Dup(ids));
                        }
                    }
                }
            }
        }
        if !to_drop.is_empty() {
            out.push(Statement::Drop(to_drop));
        }
        let out = out.into_iter().rev().collect();
        *self = out
    }
}

fn cancel_out(drop: &mut Vec<Id>, dup: &mut Vec<Id>) -> Vec<Id> {
    let all_ids = drop
        .iter()
        .chain(dup.iter())
        .map(|it| *it)
        .collect::<HashSet<_>>();
    let mut canceled = vec![];
    for id in all_ids {
        while let (Some(i), Some(j)) = (
            dup.iter().position(|it| *it == id),
            drop.iter().position(|it| *it == id),
        ) {
            dup.remove(i);
            drop.remove(j);
            canceled.push(id);
        }
    }
    canceled
}

impl Expr {
    fn collect_used_ids(&self, out: &mut Vec<Id>) {
        match self {
            Expr::Int(_) | Expr::String(_) | Expr::Symbol(_) => {}
            Expr::Closure(closure) => {
                out.append(&mut closure.captured.clone());
            }
            Expr::Map(map) => {
                for (key, value) in map {
                    out.push(*key);
                    out.push(*value);
                }
            }
            Expr::List(list) => {
                for item in list {
                    out.push(*item);
                }
            }
            Expr::Call { closure, arg } => {
                out.push(*closure);
                out.push(*arg);
            }
            Expr::Primitive { kind: _, arg } => {
                out.push(*arg);
            }
        }
    }
}
