use super::*;
use std::collections::HashSet;

impl Statements {
    pub fn optimize(&mut self) {
        self.remove_unused_statements()
    }
}

impl Statements {
    fn remove_unused_statements(&mut self) {
        let mut used = HashSet::new();
        let mut removable = HashSet::new();
        for (id, statement) in self.iter().rev() {
            if !used.contains(id) {
                if matches!(
                    statement,
                    Statement::Call { .. } | Statement::Primitive { .. }
                ) {
                    statement.collect_used_ids(&mut used);
                } else {
                    removable.insert(*id);
                }
            } else {
                statement.collect_used_ids(&mut used);
            }
        }
        for id in removable {
            self.remove(id);
        }
    }
}

impl Statement {
    fn collect_used_ids(&self, used: &mut HashSet<Id>) {
        match self {
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
            _ => {}
        }
    }
}

// fn
