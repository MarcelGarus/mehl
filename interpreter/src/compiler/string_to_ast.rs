use std::collections::HashMap;

use super::ast::{Ast, Asts};
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

/// Parses an int like `123`, `2r100100101`, or `36rax9z3l1m6`.
fn int(input: &str) -> ParseResult<u64> {
    // TODO: Return BigInt.
    // TODO: Support negative numbers?

    let (number_or_radix, input) = match raw_int(input, 10, true) {
        NotApplicable => return NotApplicable,
        Parsed(number, input) => (number, input),
        Error(error, input) => return Error(error, input),
    };
    if let Some('r') = input.chars().next() {
        let radix = number_or_radix as usize;
        // TODO: Check that radix is valid.
        raw_int(&input[1..], radix, false)
    } else {
        Parsed(number_or_radix, input)
    }
}
fn raw_int(input: &str, radix: usize, allow_trailing_r: bool) -> ParseResult<u64> {
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
    !c.is_whitespace() && "[]{}(),:".chars().all(|it| it != c)
}

fn symbol(input: &str) -> ParseResult<String> {
    if let Some(':') = input.chars().next() {
        match identifier(&input[1..]) {
            NotApplicable => Parsed("".into(), &input[1..]),
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
                Parsed(asts.into_vec(), &input[1..])
            } else {
                Error(format!("Expected code to end here."), input)
            }
        }
        Error(err, input) => Error(err, input),
    }
}

fn let_(input: &str) -> ParseResult<String> {
    if !input.starts_with("=>") {
        return NotApplicable;
    }
    let input = remove_leading_whitespace_and_comments(&input[2..]);
    match identifier(input) {
        NotApplicable => Error(format!("Expected identifier here."), input),
        Parsed(name, input) => Parsed(name, input),
        Error(err, input) => Error(err, input),
    }
}

fn fun(input: &str) -> ParseResult<String> {
    if !input.starts_with("->") {
        return NotApplicable;
    }
    let input = remove_leading_whitespace_and_comments(&input[2..]);
    match identifier(input) {
        NotApplicable => Error(format!("Expected identifier here."), input),
        Parsed(name, input) => Parsed(name, input),
        Error(err, input) => Error(err, input),
    }
}

fn ast(input: &str) -> ParseResult<Ast> {
    let parsers: Vec<fn(&str) -> ParseResult<Ast>> = vec![
        |input| int(input).map_result(|int| Ast::Int(int as i64)),
        |input| string(input).map_result(|string| Ast::String(string)),
        |input| symbol(input).map_result(|symbol| Ast::Symbol(symbol)),
        |input| list(input).map_result(|list| Ast::List(list)),
        |input| map(input).map_result(|map| Ast::Map(map)),
        |input| code(input).map_result(|code| Ast::Code(code.into())),
        |input| let_(input).map_result(|name| Ast::Let(name)),
        |input| fun(input).map_result(|name| Ast::Fun(name)),
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
    Parsed(asts.into(), remove_leading_whitespace_and_comments(input))
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

pub trait ParseStringToAsts {
    fn parse_to_asts(&self) -> Result<Asts, String>;
}
impl ParseStringToAsts for str {
    fn parse_to_asts(&self) -> Result<Asts, String> {
        match asts(self) {
            ParseResult::NotApplicable => panic!("ASTs should never be not applicable."),
            ParseResult::Parsed(asts, input) => {
                if input.is_empty() {
                    Ok(asts)
                } else {
                    Err(format!(
                        "Couldn't parse everything.\nASTs so far: {}\nRest of the input: {}",
                        &asts, input
                    ))
                }
            }
            ParseResult::Error(error, input) => Err(format!(
                "Couldn't parse code: {}\nRest of the input: {}",
                error, input
            )),
        }
    }
}
