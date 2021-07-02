mod expr;
mod parser;
mod runner;

fn main() {
    println!("Running Mehl interpreter on test.mehl.");
    let code = std::fs::read_to_string("test.mehl").expect("File not found.");
    let ast = match parser::parse(&code) {
        Ok(ast) => ast,
        Err(err) => {
            print!("Error while parsing ast: {}", err);
            return;
        }
    };
    println!(
        "Ast: {}",
        itertools::join(ast.iter().map(|a| format!("{}", a)), " ")
    );
    let expression = runner::run(ast);
    println!("Expression: {}", expression);
}
