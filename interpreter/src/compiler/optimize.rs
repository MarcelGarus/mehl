use super::*;
use std::collections::{HashMap, HashSet};

impl Ir {
    pub fn optimize(&mut self) {
        let code = &mut self.code;
        code.deduplicate_statements();
        code.remove_unused_statements();
        code.make_primitives_concrete();
        code.remove_unused_statements();
        println!("{}", code);
        code.inline_code();
        code.inline_code();
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
            Statement::Call { .. } => false,
            Statement::Primitive { kind, .. } => match kind {
                Primitive::Magic => false,
                Primitive::Add => true,
                Primitive::Print => false,
            },
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
            Statement::Primitive { arg, .. } => {
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
        // Note: Not using a for with range here because the length is still
        // changing while we iterate.
        let mut id = self.in_;
        while id < self.next_id() - 1 {
            id += 1;
            let statement = self.get_mut(id).unwrap();
            if !statement.is_pure() {
                continue;
            }
            if let Statement::Code(code) = statement {
                code.deduplicate_statements_helper(pure_statements.clone());
            }

            match pure_statements.get(&statement) {
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

/// If we can statically determine, which primitive will be called, make it
/// concrete.

impl Code {
    fn make_primitives_concrete(&mut self) {
        self.make_primitives_concrete_helper(&im::HashMap::new())
    }

    fn make_primitives_concrete_helper(&mut self, statements: &im::HashMap<Id, Statement>) {
        let mut statements = statements.clone();
        for id in self.in_ + 1..self.next_id() {
            let statement = self.get_mut(id).unwrap();

            if let Statement::Code(code) = statement {
                code.make_primitives_concrete_helper(&statements);
            }

            statements.insert(id, statement.clone());
            let statement = statement.clone();
            if let Some(concrete) = statement.try_making_concrete(&statements) {
                let mut updates = HashMap::new();
                updates.insert(id, id);
                self.replace_range(id, 1, vec![concrete], updates);
            }
        }
    }
}

impl Statement {
    fn try_making_concrete(&self, statements: &im::HashMap<Id, Statement>) -> Option<Statement> {
        let arg = match self {
            Statement::Primitive {
                kind: Primitive::Magic,
                arg,
            } => *arg,
            _ => return None,
        };
        let arg = match statements.get(&arg).unwrap() {
            Statement::List(list) => list,
            _ => return None,
        };
        if arg.len() != 2 {
            return None;
        }
        let primitive = arg[0];
        let arg = arg[1];
        let primitive = match statements.get(&primitive).unwrap().clone() {
            Statement::Symbol(symbol) => symbol,
            _ => return None,
        };

        let kind = match primitive.as_str() {
            "add" => Primitive::Add,
            "print" => Primitive::Print,
            _ => return None,
        };
        Some(Statement::Primitive { kind, arg })
    }
}

/// A call to some code can instead insert the code right there and adjust the
/// IDs.

impl Code {
    fn inline_code(&mut self) {
        self.inline_code_helper(&im::HashMap::new());
    }
    fn inline_code_helper(&mut self, statements: &im::HashMap<Id, Statement>) {
        let mut statements = statements.clone();
        for id in self.in_ + 1..self.next_id() {
            if let Statement::Call { fun, arg } = self.get(id).unwrap().clone() {
                if let Statement::Code(code) = statements.get(&fun).unwrap() {
                    let mut code = code.clone();
                    let in_ = code.in_;
                    let shift = id - in_ - 1;
                    code.replace_ids(&|it| {
                        if it == in_ {
                            arg
                        } else if it > in_ {
                            it + shift
                        } else {
                            it
                        }
                    });

                    let mut updates = HashMap::new();
                    updates.insert(id, code.out + shift);

                    self.replace_range(
                        id,
                        1,
                        code.iter()
                            .map(|(_, statement)| statement)
                            .collect::<Vec<_>>(),
                        updates,
                    );
                }
                continue;
            }

            if let Statement::Code(code) = self.get_mut(id).unwrap() {
                code.inline_code_helper(&statements);
            }

            statements.insert(id, self.get(id).unwrap().clone());
        }
    }
}

// Execute pure primitives.
// Intern symbols.
// Split primitive calls with known return value into two statements.
// Remove pure statements before panic.
