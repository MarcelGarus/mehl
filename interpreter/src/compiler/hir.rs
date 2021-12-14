use super::primitives::PrimitiveKind;
use itertools::Itertools;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::fmt;

pub type Id = u32;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Statement {
    Int(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Id, Id>),
    List(Vec<Id>),
    Code(Code),
    Call {
        fun: Id,
        arg: Id,
    },
    Primitive {
        kind: Option<PrimitiveKind>,
        arg: Id,
    },
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Code {
    pub in_: Id,
    pub out: Id,
    statements: Vec<Statement>,
}

impl Statement {
    pub fn unit() -> Self {
        Self::Symbol("".into())
    }
}

impl Code {
    pub fn new(in_: Id, out: Id) -> Self {
        Self {
            in_,
            out,
            statements: vec![],
        }
    }
    pub fn next_id(&self) -> Id {
        self.in_ + (self.statements.len() as u32) + 1
    }
    pub fn push_without_changing_dot(&mut self, statement: Statement) -> Id {
        let id = self.next_id();
        self.statements.push(statement);
        id
    }
    pub fn push(&mut self, statement: Statement) -> Id {
        let id = self.push_without_changing_dot(statement);
        self.out = id;
        id
    }
    pub fn get(&self, id: Id) -> Option<&Statement> {
        let index = id as i64 - self.in_ as i64 - 1;
        if index < 0 {
            None
        } else {
            self.statements.get(index as usize)
        }
    }
    pub fn get_mut(&mut self, id: Id) -> Option<&mut Statement> {
        let index = id as i64 - self.in_ as i64 - 1;
        if index < 0 {
            None
        } else {
            self.statements.get_mut(index as usize)
        }
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Id, Statement)> {
        Iter {
            shift: self.in_ + 1,
            inner: self.statements.clone().into_iter().enumerate(),
        }
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (Id, &mut Statement)> {
        Iter {
            shift: self.in_ + 1,
            inner: self.statements.iter_mut().enumerate(),
        }
    }
    pub fn child(&self, out: Id) -> Self {
        Self::new(self.next_id(), out)
    }
    pub fn child_identity(&self) -> Self {
        Self::new(self.next_id(), self.next_id())
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
            Statement::Code(Code {
                in_,
                out,
                statements,
            }) => {
                in_.hash(state);
                out.hash(state);
                statements.hash(state);
            }
            Statement::Call { fun, arg } => {
                fun.hash(state);
                arg.hash(state);
            }
            Statement::Primitive { kind, arg } => {
                kind.hash(state);
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
            Statement::Code(code) => {
                write!(
                    f,
                    "code [\n{}\n]",
                    code.to_string()
                        .lines()
                        .map(|line| format!("  {}", line))
                        .join("\n"),
                )
            }
            Statement::Call { fun, arg } => write!(f, "call {}({})", fun, arg),
            Statement::Primitive { kind, arg } => write!(f, "primitive {:?} {}", kind, arg),
        }
    }
}
impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "in: {}\n", self.in_)?;
        for (id, action) in self.iter() {
            write!(f, "{} = {}\n", id, action)?;
        }
        write!(f, "out: {}\n", self.out)?;
        Ok(())
    }
}

impl Code {
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
    /// 6 = primitive_print 5
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
        debug!(
            "Optimizer: Replacing {} len {} with {:?}. Replacements: {:?}",
            start, length, replacement, reference_replacements
        );
        let mut statements = vec![];

        let start = start;
        let end = start + length as u32;
        let start_index = start as usize - self.in_ as usize - 1;
        let end_index = end as usize - self.in_ as usize - 1;

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
        let transform = |id| {
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
        };
        for statement in &mut self.statements[end_index as usize..] {
            let mut statement = statement.clone();
            statement.replace_ids(&transform);
            statements.push(statement);
        }

        self.statements = statements;
        self.in_ = transform(self.in_);
        self.out = transform(self.out);

        debug!("Now the HIR is this: {}", self);
    }

    pub fn replace_ids<F: Fn(Id) -> Id>(&mut self, transform: &F) {
        self.in_ = transform(self.in_);
        self.out = transform(self.out);
        for (_, statement) in self.iter_mut() {
            statement.replace_ids(transform);
        }
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
            Statement::Code(code) => {
                code.in_ = transform(code.in_);
                code.out = transform(code.out);
                for (_, statement) in code.iter_mut() {
                    statement.replace_ids(transform);
                }
            }
            Statement::Call { fun, arg } => {
                *fun = transform(*fun);
                *arg = transform(*arg);
            }
            Statement::Primitive { arg, .. } => {
                *arg = transform(*arg);
            }
        }
    }
}

impl Statement {
    pub fn collect_used_ids(&self, used: &mut HashSet<Id>) {
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
