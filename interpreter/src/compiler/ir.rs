use itertools::Itertools;
use std::{collections::HashMap, fmt, mem};

pub type Id = u32;

#[derive(Clone, PartialEq, Eq)]
pub enum Statement {
    Int(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Id, Id>),
    List(Vec<Id>),
    Code { statements: Statements, out: Id },
    Call { fun: Id, arg: Id },
    Primitive { arg: Id },
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Statements {
    first_id: Id,
    statements: Vec<Statement>,
}

pub struct Ir {
    pub statements: Statements,
    pub out: Id,
}

impl Statements {
    pub fn new(first_id: Id) -> Self {
        Self {
            first_id,
            statements: vec![],
        }
    }
    pub fn next_id(&self) -> Id {
        self.first_id + (self.statements.len() as u32)
    }
    pub fn push(&mut self, statement: Statement) -> Id {
        self.statements.push(statement);
        self.next_id() - 1
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Id, &Statement)> {
        Iter {
            shift: self.first_id,
            inner: self.statements.iter().enumerate(),
        }
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (Id, &mut Statement)> {
        Iter {
            shift: self.first_id,
            inner: self.statements.iter_mut().enumerate(),
        }
    }
    pub fn child(&self) -> Self {
        Self::new(self.next_id())
    }
}
pub struct Iter<T, I: Iterator<Item = (usize, T)>> {
    shift: Id,
    inner: I,
}
impl<T, I: Iterator<Item = (usize, T)>> Iterator for Iter<T, I> {
    type Item = (Id, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(id, statement)| (id as u32 + self.shift, statement))
    }
}
impl<T, I: DoubleEndedIterator<Item = (usize, T)>> DoubleEndedIterator for Iter<T, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|(id, statement)| (id as u32 + self.shift, statement))
    }
}

impl std::hash::Hash for Statement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Statement::Int(int) => int.hash(state),
            Statement::String(string) => string.hash(state),
            Statement::Symbol(symbol) => symbol.hash(state),
            Statement::Map(map) => {
                let mut hash = 0;
                for (key, value) in map {
                    hash ^= key;
                    hash ^= value;
                }
                hash.hash(state);
            }
            Statement::List(list) => list.hash(state),
            Statement::Code { statements, out } => {
                statements.hash(state);
                out.hash(state);
            }
            Statement::Call { fun, arg } => {
                fun.hash(state);
                arg.hash(state);
            }
            Statement::Primitive { arg } => {
                arg.hash(state);
            }
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Int(int) => write!(f, "int {}", int),
            Statement::String(string) => write!(f, "string {:?}", string),
            Statement::Symbol(symbol) => write!(f, "symbol :{}", symbol),
            Statement::Map(map) => write!(f, "map {:?}", map),
            Statement::List(list) => write!(f, "list {:?}", list),
            Statement::Code { statements, out } => {
                write!(
                    f,
                    "code [\n  in: {}\n{}  out: {}\n]",
                    statements.first_id,
                    statements
                        .to_string()
                        .lines()
                        .map(|line| format!("  {}\n", line))
                        .join(""),
                    out
                )
            }
            Statement::Call { fun, arg } => write!(f, "call {}({})", fun, arg),
            Statement::Primitive { arg } => write!(f, "primitive {}", arg),
        }
    }
}
impl fmt::Display for Statements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (id, action) in self.iter() {
            write!(f, "{} = {}\n", id, action)?;
        }
        Ok(())
    }
}

impl fmt::Display for Ir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}out: {}", self.statements, self.out)?;
        Ok(())
    }
}

impl Statements {
    /// Replaces a range of the statements with some other statements. Updates
    /// all later references into the range using the `reference_replacements`.
    ///
    /// # Example
    ///
    /// Given this code:
    ///
    /// ```txt
    /// 0 = symbol :
    /// 1 = number 1
    /// 2 = number 2
    /// 3 = primitive_add 1 2
    /// 4 = primitive_print 3
    /// 5 = symbol :foo
    /// 6 = primmitive_print 5
    /// ```
    ///
    /// Calling `replace_range(1, 3, [number 3], {3, 1})` turns it into this:
    ///
    /// ```txt
    /// 0 = symbol :
    /// 1 = number 3
    /// 2 = primitive_print 1
    /// 3 = symbol :foo
    /// 4 = primitive_print 3
    /// ```
    pub fn replace_range(
        &mut self,
        start: Id,
        length: usize,
        replacement: Vec<Statement>,
        reference_replacements: HashMap<Id, Id>,
    ) {
        let mut statements = vec![];

        let start = start;
        let end = start + length as u32;
        let start_index = start as usize - self.first_id as usize;
        let end_index = end as usize - self.first_id as usize;

        // The statements before the replaced part stay the same.
        for statement in &self.statements[0..start_index] {
            statements.push(statement.clone());
        }

        // The replaced part gets ignored, we use the replacement instead.
        for statement in &replacement {
            statements.push(statement.clone());
        }

        // The statements after that need to get their IDs replaced. IDs that
        // reference statements before the replaced ones stay the same. IDs that
        // reference into the replaced range get replaced according to the
        // `reference_replacements`. IDs that reference statements after the
        // replaced range get shifted – the replacement may have a different
        // length than the replaced statements.
        let shift = replacement.len() as isize - length as isize;
        for statement in &mut self.statements[end_index as usize..] {
            let mut statement = statement.clone();
            statement.replace_ids(&|id| {
                if id < start {
                    id
                } else if id >= end {
                    (id as isize + shift) as u32
                } else {
                    *reference_replacements.get(&id).expect(&format!(
                        "Reference to ID {} in replaced range with no replacement.",
                        id
                    ))
                }
            });
            statements.push(statement);
        }

        mem::replace(&mut self.statements, statements);
    }
}

impl Statement {
    fn replace_ids<F: Fn(Id) -> Id>(&mut self, transform: &F) {
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
            Statement::Code { statements, out } => {
                let first_id = statements.first_id;
                mem::replace(&mut statements.first_id, transform(first_id));
                // *statements.first_id = transform(*statements.first_id);
                *out = transform(*out);
                for (_, statement) in statements.iter_mut() {
                    statement.replace_ids(transform);
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
