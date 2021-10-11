use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, fmt};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Ast {
    Number(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Ast, Ast>),
    List(Vec<Ast>),
    Code(Vec<Ast>),
    Name(String),
}
impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ast::Number(number) => write!(f, "{}", number),
            Ast::String(string) => write!(f, "{:?}", string),
            Ast::Symbol(symbol) => write!(f, ":{}", symbol),
            Ast::Map(map) => write!(
                f,
                "{{{}}}",
                itertools::join(
                    map.iter().map(|(key, value)| format!("{}, {}", key, value)),
                    ", "
                )
            ),
            Ast::List(list) => write!(
                f,
                "({})",
                itertools::join(list.iter().map(|item| format!("{}", item)), ", ")
            ),
            Ast::Code(code) => write!(
                f,
                "[{}]",
                itertools::join(code.iter().map(|item| format!("{}", item)), " ")
            ),
            Ast::Name(name) => write!(f, "{}", name),
        }
    }
}

pub fn format_code(asts: &[Ast]) -> String {
    itertools::join(asts.into_iter().map(|ast| format!("{}", ast)), " ")
}

impl Hash for Ast {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Ast::Number(number) => number.hash(state),
            Ast::String(string) => string.hash(state),
            Ast::Symbol(symbol) => symbol.hash(state),
            Ast::Map(map) => {
                let mut h = 0;

                for element in map.iter() {
                    let mut hasher = DefaultHasher::new();
                    element.hash(&mut hasher);
                    h ^= hasher.finish();
                }

                state.write_u64(h);
            }
            Ast::List(list) => list.hash(state),
            Ast::Code(code) => code.hash(state),
            Ast::Name(name) => name.hash(state),
        }
    }
}
impl Ast {
    pub fn unit() -> Self {
        Self::Symbol("".into())
    }
    pub fn as_number(self) -> Option<i64> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }
    pub fn as_string(self) -> Option<String> {
        match self {
            Self::String(string) => Some(string),
            _ => None,
        }
    }
    pub fn as_symbol(self) -> Option<String> {
        match self {
            Self::Symbol(symbol) => Some(symbol),
            _ => None,
        }
    }
    pub fn as_map(self) -> Option<HashMap<Self, Self>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }
    pub fn as_list(self) -> Option<Vec<Self>> {
        match self {
            Self::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn as_code(self) -> Option<Vec<Self>> {
        match self {
            Self::Code(code) => Some(code),
            _ => None,
        }
    }
    pub fn as_name(self) -> Option<String> {
        match self {
            Self::Name(name) => Some(name),
            _ => None,
        }
    }
}

pub trait MapGetStrSymbolExt {
    fn get_symbol(&self, key: &str) -> Option<&Ast>;
}
impl MapGetStrSymbolExt for HashMap<Ast, Ast> {
    fn get_symbol(&self, key: &str) -> Option<&Ast> {
        self.get(&Ast::Symbol(key.into()))
    }
}

mod parse {
    use super::*;
    use nom::{
        branch::alt,
        bytes::complete::{is_not, take_while1},
        bytes::complete::{tag, take_while, take_while_m_n},
        character::complete::char,
        combinator::{map_res, opt},
        multi::{many1, separated_list},
        sequence::{delimited, pair, tuple},
        IResult,
    };

    fn number(input: &str) -> IResult<&str, Ast> {
        let (input, number) = map_res(take_while(|c: char| c.is_digit(10)), |digits| {
            i64::from_str_radix(digits, 10)
        })(input)?;
        Ok((input, Ast::Number(number)))
    }

    fn string(input: &str) -> IResult<&str, Ast> {
        let (input, string) = delimited(char('"'), is_not("\""), char('"'))(input)?;
        Ok((input, Ast::String(string.to_owned())))
    }

    fn identifier(input: &str) -> IResult<&str, String> {
        let (input, identifier) = take_while_m_n(1, 10000, |c: char| {
            !c.is_whitespace() && !matches!(c, ',' | '(' | ')')
        })(input)?;
        Ok((input, identifier.to_owned()))
    }

    fn symbol(input: &str) -> IResult<&str, Ast> {
        let (input, _) = tag(":")(input)?;
        let (input, identifier) = opt(identifier)(input)?;
        Ok((input, Ast::Symbol(identifier.unwrap_or("".into()))))
    }

    fn name(input: &str) -> IResult<&str, Ast> {
        let (input, name) = identifier(input)?;
        Ok((input, Ast::Name(name)))
    }

    fn list(input: &str) -> IResult<&str, Ast> {
        let (input, list) = delimited(
            char('('),
            separated_list(pair(char(','), whitespace_or_comment), ast),
            char(')'),
        )(input)?;
        Ok((input, Ast::List(list)))
    }

    fn code(input: &str) -> IResult<&str, Ast> {
        let (input, asts) = delimited(char('['), asts, char(']'))(input)?;
        Ok((input, Ast::Code(asts)))
    }

    fn map(input: &str) -> IResult<&str, Ast> {
        let (input, code_literals) = delimited(
            pair(char('{'), opt(whitespace_or_comment)),
            separated_list(pair(char(','), whitespace_or_comment), ast),
            pair(whitespace_or_comment, char('}')),
        )(input)?;
        if code_literals.len() % 2 == 1 {
            return Err(nom::Err::Failure((
                "There should be an even number of items inside maps.",
                nom::error::ErrorKind::LengthValue,
            )));
        }
        let mut map: HashMap<Ast, Ast> = HashMap::new();
        let mut code_literals = code_literals.into_iter();
        while let Some(key) = code_literals.next() {
            let value = code_literals
                .next()
                .expect("Odd number of code literals in map literal.");
            map.insert(key, value);
        }
        Ok((input, Ast::Map(map)))
    }

    fn ast(input: &str) -> IResult<&str, Ast> {
        let (input, ast) = alt((number, string, symbol, map, list, code, name))(input)?;
        Ok((input, ast))
    }

    fn comment(input: &str) -> IResult<&str, ()> {
        let (input, _) = delimited(char('#'), is_not("\n"), char('\n'))(input)?;
        Ok((input, ()))
    }

    fn whitespace(input: &str) -> IResult<&str, ()> {
        let (input, _) = take_while1(|c: char| c.is_whitespace())(input)?;
        Ok((input, ()))
    }

    fn whitespace_or_comment(input: &str) -> IResult<&str, ()> {
        let (input, _) = many1(alt((whitespace, comment)))(input)?;
        Ok((input, ()))
    }

    pub fn asts(input: &str) -> IResult<&str, Vec<Ast>> {
        let (input, _) = opt(whitespace_or_comment)(input)?;
        let (input, asts) = separated_list(whitespace_or_comment, ast)(input)?;
        let (input, _) = opt(whitespace_or_comment)(input)?;
        Ok((input, asts))
    }
}

impl Ast {
    pub fn parse_all(input: &str) -> Result<Vec<Ast>, String> {
        match parse::asts(input) {
            Ok((input, asts)) => {
                if input.is_empty() {
                    Ok(asts)
                } else {
                    Err(format!(
                        "Couldn't parse everything.\nASTs so far: {}\nRest of the input:\n{}",
                        format_code(&asts),
                        input
                    ))
                }
            }
            Err(err) => Err(format!("Error while parsing {:?}", err)),
        }
    }
}
