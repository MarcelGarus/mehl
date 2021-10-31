use super::*;
use std::collections::{HashMap, HashSet};

impl Ir {
    pub fn optimize(&mut self) {
        self.remove_unused_statements();
        self.deduplicate_statements();
        self.remove_unused_statements();
    }
}

impl Ir {
    /// Statements that have no effect except creating a value, but where the
    /// result is not used, can be safely removed.
    /// For this optimization to work, code may not define ids that have been
    /// defined before ("shadowing").
    fn remove_unused_statements(&mut self) {
        let mut used = HashSet::new();
        let mut removable = HashSet::new();

        used.insert(self.out);
        for (id, statement) in self.statements.iter().rev() {
            if !used.contains(id) {
                if statement.is_pure() {
                    statement.collect_used_ids(&mut used);
                } else {
                    removable.insert(*id);
                }
            } else {
                statement.collect_used_ids(&mut used);
            }
        }
        for id in removable {
            self.statements.remove(id);
        }
    }
}

impl Statement {
    fn is_pure(&self) -> bool {
        match self {
            Statement::Int(_)
            | Statement::String(_)
            | Statement::Symbol(_)
            | Statement::Map(_)
            | Statement::List(_)
            | Statement::Code { .. } => true,
            Statement::Call { .. } | Statement::Primitive { .. } => false,
        }
    }
}

impl Statement {
    fn collect_used_ids(&self, used: &mut HashSet<Id>) {
        match self {
            Statement::Int(_) | Statement::String(_) | Statement::Symbol(_) => {}
            Statement::Map(map) => {
                for (key, value) in map {
                    used.insert(*key);
                    used.insert(*value);
                }
            }
            Statement::List(list) => {
                for item in list {
                    used.insert(*item);
                }
            }
            Statement::Code {
                in_,
                out,
                statements,
            } => {
                used.insert(*in_);
                used.insert(*out);
                for (_, statement) in statements.iter() {
                    statement.collect_used_ids(used);
                }
            }
            Statement::Call { fun, arg } => {
                used.insert(*fun);
                used.insert(*arg);
            }
            Statement::Primitive { arg } => {
                used.insert(*arg);
            }
        }
    }
}

impl Ir {
    // Deduplicates pure statements.
    fn deduplicate_statements(&mut self) {
        let mut pure_statements = HashMap::new();
        let mut replace = HashMap::new();

        for (id, statement) in self.statements.iter_mut() {
            if statement.is_pure() {
                let statement = statement.clone();
                match pure_statements.get(&statement) {
                    Some(existing_id) => {
                        replace.insert(*id, *existing_id);
                    }
                    None => {
                        pure_statements.insert(statement.clone(), *id);
                    }
                }
            }
            for (old, new) in replace.iter() {
                statement.replace_id_usage(*old, *new);
            }
        }
    }
}

impl Statement {
    fn replace_id_usage(&mut self, old: Id, new: Id) {
        let transform = |id| {
            if id == old {
                new
            } else {
                id
            }
        };
        match self {
            Statement::Int(_) | Statement::String(_) | Statement::Symbol(_) => {}
            Statement::Map(map) => {
                let mut new_map = HashMap::new();
                for (key, value) in map.iter() {
                    new_map.insert(transform(*key), transform(*value));
                }
                *map = new_map;
            }
            Statement::List(list) => {
                let new_list = list.into_iter().map(|id| transform(*id)).collect();
                *list = new_list;
            }
            Statement::Code {
                in_,
                statements,
                out,
            } => {
                *in_ = transform(*in_);
                *out = transform(*out);
                for (_, statement) in statements.iter_mut() {
                    statement.replace_id_usage(old, new);
                }
            }
            Statement::Call { fun, arg } => {
                *fun = transform(*fun);
                *arg = transform(*arg);
            }
            Statement::Primitive { arg } => {
                *arg = transform(*arg);
            }
        }
    }
}

// Meta: Optimize saved code.
// Inline code.
// Make primitives concrete.
// Execute pure primitives.
// Move constants out of code. Needed after inlining?
