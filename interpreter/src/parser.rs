use {
    crate::expr::*,
    nom::{
        branch::alt,
        bytes::complete::{is_not, take_while1},
        bytes::complete::{tag, take_while, take_while_m_n},
        character::complete::char,
        combinator::{map_res, opt},
        multi::{many1, separated_list},
        sequence::delimited,
        IResult,
    },
    std::collections::HashMap,
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
        !c.is_whitespace() && !matches!(c, ',' | '(' | ')' | '{' | '}' | '[' | ']' | ':')
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
    let (input, list) = delimited(char('('), separated_list(char(','), asts), char(')'))(input)?;
    Ok((input, Ast::List(list)))
}

fn code(input: &str) -> IResult<&str, Ast> {
    let (input, list) = delimited(char('['), asts, char(']'))(input)?;
    Ok((input, Ast::Code(list)))
}

fn map(input: &str) -> IResult<&str, Ast> {
    let (input, entries) = delimited(char('{'), asts, char('}'))(input)?;
    if entries.len() % 2 == 1 {
        return Err(nom::Err::Failure((
            "There should be an even number of items inside maps.",
            nom::error::ErrorKind::LengthValue,
        )));
    }
    let mut map = HashMap::new();
    for key_and_value in entries.chunks(2) {
        let key = match key_and_value.get(0).unwrap().clone() {
            Ast::Symbol(symbol) => symbol,
            _ => {
                return Err(nom::Err::Failure((
                    "Map keys need to be symbols.",
                    nom::error::ErrorKind::Fix,
                )))
            }
        };
        let value = key_and_value.get(1).unwrap().clone();
        map.insert(key, value);
    }
    Ok((input, Ast::Map(map)))
}

fn ast(input: &str) -> IResult<&str, Ast> {
    let (input, ast) = alt((number, string, symbol, name, list, map, code))(input)?;
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

fn asts(input: &str) -> IResult<&str, Vec<Ast>> {
    let (input, _) = opt(whitespace_or_comment)(input)?;
    let (input, asts) = separated_list(whitespace_or_comment, ast)(input)?;
    let (input, _) = opt(whitespace_or_comment)(input)?;
    Ok((input, asts))
}

pub fn parse(input: &str) -> Result<Vec<Ast>, String> {
    match asts(input) {
        Ok((input, asts)) => {
            if input.is_empty() {
                Ok(asts)
            } else {
                Err(format!(
                    "Couldn't parse everything.\nASTs so far: {:?}\nRest of the input: {}",
                    asts, input
                ))
            }
        }
        Err(err) => Err(format!("Error while parsing {:?}", err)),
    }
}
