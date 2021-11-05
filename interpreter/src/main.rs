mod ast;
mod compiler;
mod runner;

use ast::*;
use clap::{App, SubCommand};
use colored::Colorize;
use compiler::*;
use lspower::jsonrpc::Result;
use lspower::lsp::*;
use lspower::{Client, LanguageServer, LspService, Server};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Warn,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let matches = App::new("Mehl")
        .version("0.0.0")
        .author("Marcel Garus <marcel.garus@gmail.com>")
        .about("Mehl language utility")
        .subcommand(SubCommand::with_name("run").about("Runs a Mehl file."))
        .subcommand(SubCommand::with_name("lsp"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("run") {
        println!("Running test.mehl.");
        let core = {
            let code = std::fs::read_to_string("core.mehl").expect("File core.mehl not found.");
            match ast::Ast::parse_all(&code) {
                Ok(it) => it,
                Err(err) => panic!("Couldn't parse ASTs of core.mehl: {}", err),
            }
        };
        println!("Core parsed.");
        let user = {
            let code = std::fs::read_to_string("test.mehl").expect("File test.mehl not found.");
            match ast::Ast::parse_all(&code) {
                Ok(it) => it,
                Err(err) => panic!("Couldn't parse ASTs of test.mehl: {}", err),
            }
        };
        println!("Test parsed.");

        println!("Code: {}", ast::format_code(&user));

        let mut code = ast_to_hir::compile(user);
        println!("HIR: {}", code);
        println!("Optimizing...");
        code.optimize();
        println!("Optimized HIR: {}", code);
        let code = hir_to_lir::compile(&code);
        println!("LIR: {}", code);

        // let mut fiber = runner::Runtime::default();
        // let context = runner::Context::root(&mut fiber);
        // let context = match context.run(&mut fiber, core) {
        //     Ok(context) => context,
        //     Err(err) => panic!("The core library panicked: {}", err),
        // };
        // let context = match context.run(&mut fiber, user) {
        //     Ok(context) => context,
        //     Err(err) => {
        //         println!(
        //             "{}\n{}{}",
        //             "The program panicked.".red(),
        //             "Message: ".red(),
        //             err.to_string().bright_red().bold()
        //         );
        //         return;
        //     }
        // };

        // let output = context.dot;
        // println!(
        //     "{}\n{}{}",
        //     "The program successfully finished.".green(),
        //     "Output: ".green(),
        //     output.to_string().bright_green().bold(),
        // );
    }

    if let Some(_) = matches.subcommand_matches("lsp") {
        // println!("Running Mehl LSP. 🍞");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, messages) = LspService::new(|client| Backend { client });
        Server::new(stdin, stdout)
            .interleave(messages)
            .serve(service)
            .await;
    }
}

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[lspower::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        println!("Initialize.");
        self.client
            .log_message(MessageType::Info, "Initializing!")
            .await;
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::Info, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
