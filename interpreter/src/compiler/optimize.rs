use super::*;
use std::collections::{HashMap, HashSet};

impl Ir {
    pub fn optimize(&mut self) {
        self.remove_unused_statements();
        // self.deduplicate_statements();
        // self.remove_unused_statements();
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

/// Pure statements with an unused result can be removed.

impl Ir {
    fn remove_unused_statements(&mut self) {
        self.statements.remove_unused_statements(self.out);
    }
}

impl Statements {
    fn remove_unused_statements(&mut self, out: Id) {
        let mut used = HashSet::new();
        let mut removable = vec![];

        used.insert(out);
        for (id, statement) in self.iter_mut().rev() {
            if let Statement::Code { statements, out } = statement {
                statements.remove_unused_statements(*out);
            }
            if !used.contains(&id) {
                if statement.is_pure() {
                    removable.push(id);
                } else {
                    statement.collect_used_ids(&mut used);
                }
            } else {
                statement.collect_used_ids(&mut used);
            }
        }
        for id in removable {
            self.replace_range(id, 1, vec![], HashMap::new());
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
            Statement::Code { out, statements } => {
                // used.insert(*statements.first_id);
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

// impl Ir {
//     // Deduplicates pure statements.
//     fn deduplicate_statements(&mut self) {
//         let mut pure_statements = HashMap::new();
//         let mut replace = HashMap::new();

//         for (id, statement) in self.statements.iter_mut() {
//             if statement.is_pure() {
//                 let statement = statement.clone();
//                 match pure_statements.get(&statement) {
//                     Some(existing_id) => {
//                         replace.insert(*id, *existing_id);
//                     }
//                     None => {
//                         pure_statements.insert(statement.clone(), *id);
//                     }
//                 }
//             }
//             for (old, new) in replace.iter() {
//                 statement.replace_id_usage(*old, *new);
//             }
//         }
//     }
// }

// Meta: Optimize saved code.
// Inline code.
// Make primitives concrete.
// Execute pure primitives.
// Move constants out of code. Needed after inlining?
// Intern symbols.
