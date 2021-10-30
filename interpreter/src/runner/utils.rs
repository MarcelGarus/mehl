// use colored::Colorize;
// use im::HashMap;

// use crate::ast::Asts;

// use super::*;

// pub trait ExprMap {
//     fn get_symbol(&self, symbol: &str) -> Option<Expr>;
// }
// impl ExprMap for HashMap<Expr, Expr> {
//     fn get_symbol(&self, symbol: &str) -> Option<Expr> {
//         self.get(&Expr::Symbol(symbol.into()))
//             .map(|expr| expr.clone())
//     }
// }

// pub type RunResult = Result<Context, Expr>;
// pub fn error<A: Into<String>, B: Into<String>>(kind: A, msg: B) -> Expr {
//     Expr::List(vec![Expr::Symbol(kind.into()), Expr::String(msg.into())])
// }
// pub fn wrong_usage<I: Into<String>>(msg: I) -> Expr {
//     error::<&str, I>("wrong-usage".into(), msg)
// }
// pub fn unknown_function<I: Into<String>>(msg: I) -> Expr {
//     error::<&str, I>("unknown-fun".into(), msg)
// }

// pub trait OptionExt<T> {
//     fn needed<I: Into<String>>(self, msg: I) -> Result<T, Expr>;
// }
// impl<T> OptionExt<T> for Option<T> {
//     fn needed<I: Into<String>>(self, msg: I) -> Result<T, Expr> {
//         self.ok_or(wrong_usage(msg.into()))
//     }
// }

// impl Expr {
//     pub fn as_number(self) -> Option<i64> {
//         match self {
//             Expr::Number(number) => Some(number),
//             _ => None,
//         }
//     }
//     pub fn as_string(self) -> Option<String> {
//         match self {
//             Expr::String(string) => Some(string),
//             _ => None,
//         }
//     }
//     pub fn as_symbol(self) -> Option<String> {
//         match self {
//             Expr::Symbol(symbol) => Some(symbol),
//             _ => None,
//         }
//     }
//     pub fn as_map(self) -> Option<HashMap<Expr, Expr>> {
//         match self {
//             Expr::Map(map) => Some(map),
//             _ => None,
//         }
//     }
//     pub fn as_list(self) -> Option<Vec<Expr>> {
//         match self {
//             Expr::List(list) => Some(list),
//             _ => None,
//         }
//     }
//     pub fn as_code(self) -> Option<(Box<Context>, Asts)> {
//         match self {
//             Expr::Code { scope, asts } => Some((scope, asts)),
//             _ => None,
//         }
//     }
//     pub fn needs_number<I: Into<String>>(self, msg: I) -> Result<i64, Expr> {
//         self.as_number().ok_or(wrong_usage(msg))
//     }
//     pub fn needs_string<I: Into<String>>(self, msg: I) -> Result<String, Expr> {
//         self.as_string().ok_or(wrong_usage(msg))
//     }
//     pub fn needs_symbol<I: Into<String>>(self, msg: I) -> Result<String, Expr> {
//         self.as_symbol().ok_or(wrong_usage(msg))
//     }
//     pub fn needs_map<I: Into<String>>(self, msg: I) -> Result<HashMap<Expr, Expr>, Expr> {
//         self.as_map().ok_or(wrong_usage(msg))
//     }
//     pub fn needs_list<I: Into<String>>(self, msg: I) -> Result<Vec<Expr>, Expr> {
//         self.as_list().ok_or(wrong_usage(msg))
//     }
//     pub fn needs_code<I: Into<String>>(self, msg: I) -> Result<(Box<Context>, Asts), Expr> {
//         self.as_code().ok_or(wrong_usage(msg))
//     }
//     pub fn needs_list_of_numbers<I: Into<String>>(self, msg: I) -> Result<Vec<i64>, Expr> {
//         let msg: String = msg.into();
//         let numbers = self.needs_list(msg.clone())?;
//         for n in &numbers {
//             n.clone().needs_number(msg.clone())?;
//         }
//         Ok(numbers
//             .into_iter()
//             .map(|n| n.as_number().unwrap())
//             .collect::<Vec<_>>())
//     }
//     pub fn needs_pair<I: Into<String>>(self, msg: I) -> Result<(Expr, Expr), Expr> {
//         let msg: String = msg.into();
//         self.needs_list(msg.clone())?.needs_two_items(msg)
//     }
//     pub fn needs_pair_of_numbers<I: Into<String>>(self, msg: I) -> Result<(i64, i64), Expr> {
//         let msg: String = msg.into();
//         let (first, second) = self.needs_pair(msg.clone())?;
//         let first = first.needs_number(msg.clone())?;
//         let second = second.needs_number(msg)?;
//         Ok((first, second))
//     }
// }
// pub trait ListOfExprExt {
//     fn needs_two_items<I: Into<String>>(self, msg: I) -> Result<(Expr, Expr), Expr>;
//     fn needs_three_items<I: Into<String>>(self, msg: I) -> Result<(Expr, Expr, Expr), Expr>;
// }
// impl ListOfExprExt for Vec<Expr> {
//     fn needs_two_items<I: Into<String>>(self, msg: I) -> Result<(Expr, Expr), Expr> {
//         if self.len() != 2 {
//             return Err(wrong_usage(msg));
//         }
//         // TODO: Make this more efficient.
//         let (first, second) = self.split_at(1);
//         first.to_vec();
//         Ok((
//             first.first().unwrap().clone(),
//             second.first().unwrap().clone(),
//         ))
//     }
//     fn needs_three_items<I: Into<String>>(self, msg: I) -> Result<(Expr, Expr, Expr), Expr> {
//         if self.len() != 3 {
//             return Err(wrong_usage(msg));
//         }
//         // TODO: Make this more efficient.
//         let (first, second) = self.split_at(1);
//         let (second, third) = second.split_at(1);
//         Ok((
//             first.first().unwrap().clone(),
//             second.first().unwrap().clone(),
//             third.first().unwrap().clone(),
//         ))
//     }
// }

// pub trait FancyFunsExt {
//     fn to_fancy_string(&self) -> String;
// }
// impl FancyFunsExt for HashMap<String, Fun> {
//     fn to_fancy_string(&self) -> String {
//         itertools::join(
//             self.iter().map(|(name, fun)| {
//                 format!("{}{}", name.blue(), fun.export_level.to_string().red())
//             }),
//             ", ",
//         )
//     }
// }
