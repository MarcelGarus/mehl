use super::*;
use std::collections::{HashMap, HashSet};

impl Ir {
    pub fn optimize(&mut self) {
        let code = &mut self.code;
        code.remove_unused_statements();
        code.deduplicate_statements();
        code.remove_unused_statements();
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

impl Code {
    fn remove_unused_statements(&mut self) {
        let mut used = HashSet::new();
        let mut removable = vec![];

        used.insert(self.out);
        for (id, statement) in self.iter_mut().rev() {
            if let Statement::Code(code) = statement {
                code.remove_unused_statements();
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
        println!("Removing {:?}", removable);
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
            Statement::Code(code) => {
                used.insert(code.in_);
                used.insert(code.out);
                for (_, statement) in code.iter() {
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

/// Deduplicated pure statements can be removed, so the result of the first one
/// is reused.

impl Code {
    fn deduplicate_statements(&mut self) {
        self.deduplicate_statements_helper(im::HashMap::new());
    }

    fn deduplicate_statements_helper(&mut self, mut pure_statements: im::HashMap<Statement, Id>) {
        let mut id = self.in_;
        while id < self.next_id() - 1 {
            id += 1;
            let statement = self.get_mut(id);
            if !statement.is_pure() {
                continue;
            }
            if let Statement::Code(code) = statement {
                code.deduplicate_statements_helper(pure_statements.clone());
            }

            match pure_statements.get(statement) {
                Some(existing_id) => {
                    let mut update = HashMap::new();
                    update.insert(id, *existing_id);
                    self.replace_range(id, 1, vec![], update);
                }
                None => {
                    pure_statements.insert(statement.clone(), id);
                }
            }
        }
    }
}

// Inline code.
// Make primitives concrete.
// Execute pure primitives.
// Move constants out of code. Needed after inlining?
// Intern symbols.
