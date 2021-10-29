use colored::Colorize;
use im::HashMap;

use super::*;

pub trait ExprMap {
    fn get_symbol(&self, symbol: &str) -> Option<Expr>;
}
impl ExprMap for HashMap<Expr, Expr> {
    fn get_symbol(&self, symbol: &str) -> Option<Expr> {
        self.get(&Expr::Symbol(symbol.into()))
            .map(|expr| expr.clone())
    }
}

pub trait FancyFunsExt {
    fn to_fancy_string(&self) -> String;
}
impl FancyFunsExt for HashMap<String, Fun> {
    fn to_fancy_string(&self) -> String {
        itertools::join(
            self.iter().map(|(name, fun)| {
                format!("{}{}", name.blue(), fun.export_level.to_string().red())
            }),
            ", ",
        )
    }
}
