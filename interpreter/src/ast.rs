use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, fmt};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Ast {
    Number(i64),
    String(String),
    Symbol(String),
    Map(HashMap<Asts, Asts>),
    List(Vec<Asts>),
    Code(Asts),
    Name(String),
}
pub type Asts = Vec<Ast>;
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
                    map.iter().map(|(key, value)| format!(
                        "{}, {}",
                        format_code(&key),
                        &format_code(value)
                    )),
                    ", "
                )
            ),
            Ast::List(list) => write!(
                f,
                "({})",
                itertools::join(
                    list.iter().map(|item| format!("{}", format_code(&item))),
                    ", "
                )
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
    pub fn as_map(self) -> Option<HashMap<Asts, Asts>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }
    pub fn as_list(self) -> Option<Vec<Asts>> {
        match self {
            Self::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn as_code(self) -> Option<Asts> {
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
    use itertools::Itertools;

    #[derive(Debug)]
    pub enum ParseResult<'a, T> {
        NotApplicable,
        Parsed(T, &'a str),
        Error(String, &'a str),
    }
    use nom::{FindSubstring, InputIter};
    use ParseResult::*;
    impl<'a, T> ParseResult<'a, T> {
        fn map_result<R, P: FnOnce(T) -> R>(self, mapper: P) -> ParseResult<'a, R> {
            match self {
                NotApplicable => NotApplicable,
                Parsed(result, input) => Parsed(mapper(result), input),
                Error(error, input) => Error(error, input),
            }
        }
    }

    trait StringExt {
        fn without_first_char() {}
    }

    fn is_separator(c: char) -> bool {
        c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ',')
    }

    /// Parses a number like `123`, `2r100100101`, or `36rax9z3l1m6`.
    fn number(input: &str) -> ParseResult<u64> {
        // TODO: Return BigInt.
        // TODO: Support negative numbers?

        let (number_or_radix, input) = match raw_number(input, 10, true) {
            NotApplicable => return NotApplicable,
            Parsed(number, input) => (number, input),
            Error(error, input) => return Error(error, input),
        };
        if let Some('r') = input.chars().next() {
            let radix = number_or_radix as usize;
            // TODO: Check that radix is valid.
            raw_number(&input[1..], radix, false)
        } else {
            Parsed(number_or_radix, input)
        }
    }
    fn raw_number(input: &str, radix: usize, allow_trailing_r: bool) -> ParseResult<u64> {
        // TODO: Allow underscores.
        // TODO: Return BigInt.

        let mut input = input;
        let digits = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let digits = &digits[..radix];

        let mut digits_to_parse = vec![];
        loop {
            match input.chars().next() {
                None => break,
                Some(c) if is_separator(c) => break,
                Some(c) => {
                    if digits.contains(c) {
                        input = &input[1..];
                        digits_to_parse.push(c);
                    } else if c == 'r' && allow_trailing_r {
                        input = &input[1..];
                        break;
                    } else if digits_to_parse.is_empty() {
                        return NotApplicable;
                    } else {
                        return Error(
                            format!(
                                "The character '{}' is not a valid digit in radix {}.",
                                c, radix
                            ),
                            input,
                        );
                    }
                }
            }
        }
        if digits_to_parse.is_empty() {
            return NotApplicable;
        }
        let mut number: u64 = 0;
        while !digits_to_parse.is_empty() {
            let next_digit = digits_to_parse.remove(0);
            number = number * (radix as u64)
                + (digits.chars().position(|c| c == next_digit).unwrap() as u64);
        }
        Parsed(number, input)
    }

    /// Parses a string like `"Foo"` or `'"Foo's label said: "Foo""`.
    fn string(input: &str) -> ParseResult<String> {
        // TODO: Support inline expressions.

        let number_of_single_quotes = {
            let mut input = input.chars();
            let mut counter = 0;
            while matches!(input.next(), Some('\'')) {
                counter = counter + 1;
            }
            counter
        };
        let input = &input[number_of_single_quotes..];
        if !matches!(input.chars().next(), Some('"')) {
            return if number_of_single_quotes > 0 {
                Error("Expected double quote after single quotes.".into(), input)
            } else {
                NotApplicable
            };
        }
        let input = &input[1..];

        let ending_sequence = vec!['\"']
            .into_iter()
            .chain(itertools::repeat_n('\'', number_of_single_quotes))
            .collect::<String>();
        match input.find_substring(&ending_sequence) {
            Some(end) => {
                let string_content = input[..end].to_owned();
                Parsed(string_content, &input[end + ending_sequence.len()..])
            }
            None => Error("String started, but didn't end.".into(), ""),
        }
    }

    fn identifier(input: &str) -> ParseResult<String> {
        if input
            .chars()
            .next()
            .map(is_valid_identifier_char)
            .unwrap_or(false)
        {
            let identifier = input
                .chars()
                .take_while(|c| is_valid_identifier_char(*c))
                .collect::<String>();
            let len = identifier.len();
            Parsed(identifier, &input[len..])
        } else {
            NotApplicable
        }
    }
    fn is_valid_identifier_char(c: char) -> bool {
        !c.is_whitespace() && !"[]{}(),:".chars().any(|it| it == c)
    }

    fn symbol(input: &str) -> ParseResult<String> {
        if let Some(':') = input.chars().next() {
            match identifier(&input[1..]) {
                NotApplicable => Parsed("".into(), input),
                Parsed(identifier, input) => Parsed(identifier, input),
                Error(_, _) => panic!("The identifier parser should never error."),
            }
        } else {
            NotApplicable
        }
    }

    fn list(input: &str) -> ParseResult<Vec<Asts>> {
        if let Some('(') = input.chars().next() {
        } else {
            return NotApplicable;
        }
        let mut input = &input[1..];
        let mut items = vec![];
        loop {
            input = remove_leading_whitespace_and_comments(input);
            if let Some(')') = input.chars().next() {
                return Parsed(items, &input[1..]);
            }
            match asts(input) {
                NotApplicable => panic!("ASTs parser should never be not applicable."),
                Parsed(asts, rest) => {
                    if asts.len() == 0 {
                        return Error("Expected a list item here.".into(), input);
                    }
                    items.push(asts);
                    input = rest;
                    if let Some(',') = input.chars().next() {
                        input = &input[1..];
                    }
                }
                Error(err, input) => return Error(err, input),
            }
        }
    }

    fn map(input: &str) -> ParseResult<HashMap<Asts, Asts>> {
        if let Some('{') = input.chars().next() {
        } else {
            return NotApplicable;
        }
        let map_start_input = input;
        let mut input = &input[1..];
        let mut items = vec![];
        loop {
            input = remove_leading_whitespace_and_comments(input);
            if let Some('}') = input.chars().next() {
                if items.len() % 2 == 0 {
                    let mut map = HashMap::new();
                    for mut chunk in &items.into_iter().chunks(2) {
                        let key = chunk.next().unwrap();
                        let value = chunk.next().unwrap();
                        map.insert(key, value);
                    }
                    return Parsed(map, &input[1..]);
                } else {
                    return Error(
                        "Maps have to contain an even number of elements.".into(),
                        map_start_input,
                    );
                }
            }
            match asts(input) {
                NotApplicable => panic!("ASTs parser should never be not applicable."),
                Parsed(asts, rest) => {
                    items.push(asts);
                    input = rest;
                    if let Some(',') = input.chars().next() {
                        input = &input[1..];
                    }
                }
                Error(err, input) => return Error(err, input),
            }
            if let Some(',') = input.chars().next() {
                input = &input[1..];
            }
        }
    }

    fn code(input: &str) -> ParseResult<Vec<Ast>> {
        if let Some('[') = input.chars().next() {
        } else {
            return NotApplicable;
        }
        match asts(&input[1..]) {
            NotApplicable => panic!("ASTs parser should never be not applicable."),
            Parsed(asts, input) => {
                if let Some(']') = input.chars().next() {
                    Parsed(asts, &input[1..])
                } else {
                    Error(format!("Expected code to end here."), input)
                }
            }
            Error(err, input) => Error(err, input),
        }
    }

    fn ast(input: &str) -> ParseResult<Ast> {
        let parsers: Vec<fn(&str) -> ParseResult<Ast>> = vec![
            |input| number(input).map_result(|number| Ast::Number(number as i64)),
            |input| string(input).map_result(|string| Ast::String(string)),
            |input| symbol(input).map_result(|symbol| Ast::Symbol(symbol)),
            |input| list(input).map_result(|list| Ast::List(list)),
            |input| map(input).map_result(|map| Ast::Map(map)),
            |input| code(input).map_result(|code| Ast::Code(code)),
            |input| identifier(input).map_result(|name| Ast::Name(name)),
        ];
        for parser in parsers {
            let result = parser(input);
            if let NotApplicable = result {
                continue;
            } else {
                return result;
            }
        }
        NotApplicable
    }

    pub fn asts(input: &str) -> ParseResult<Asts> {
        let mut input = input;
        let mut asts = vec![];
        loop {
            input = remove_leading_whitespace_and_comments(input);
            match ast(input) {
                NotApplicable => break,
                Parsed(ast, rest) => {
                    asts.push(ast);
                    input = rest;
                }
                Error(error, input) => return Error(error, input),
            }
        }
        Parsed(asts, remove_leading_whitespace_and_comments(input))
    }

    fn remove_leading_whitespace_and_comments(input: &str) -> &str {
        let mut input = input;
        loop {
            let old_input = input;
            input = input.trim_start();
            if let Some('#') = input.chars().next() {
                let end_of_line = input.position(|c| c == '\n').unwrap_or_else(|| input.len());
                input = &input[end_of_line..];
                continue;
            }
            if input.len() == old_input.len() {
                return input;
            }
        }
    }
}

impl Ast {
    pub fn parse_all(input: &str) -> Result<Asts, String> {
        match parse::asts(input) {
            parse::ParseResult::NotApplicable => panic!("ASTs should never be not applicable."),
            parse::ParseResult::Parsed(asts, input) => {
                if input.is_empty() {
                    Ok(asts)
                } else {
                    Err(format!(
                        "Couldn*t parse everything.\nASTs so far: {}\nRest of the input: {}",
                        format_code(&asts),
                        input
                    ))
                }
            }
            parse::ParseResult::Error(error, input) => Err(format!(
                "Couldn't parse code: {}\nRest of the input: {}",
                error, input
            )),
        }
    }
}
