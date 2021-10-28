mod ast;
mod runner;

use ast::*;
use clap::{App, SubCommand};
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
            match Ast::parse_all(&code) {
                Ok(it) => it,
                Err(err) => panic!("Couldn't parse ASTs of core.mehl: {}", err),
            }
        };
        println!("Core parsed.");
        let user = {
            let code = std::fs::read_to_string("test.mehl").expect("File test.mehl not found.");
            match Ast::parse_all(&code) {
                Ok(it) => it,
                Err(err) => panic!("Couldn't parse ASTs of test.mehl: {}", err),
            }
        };
        println!("Test parsed.");

        println!("Code: {}", format_code(&user));
        let mut fiber = runner::Fiber::new();
        let expression = runner::Context::root(&mut fiber)
            .run(&mut fiber, core)
            .run(&mut fiber, user)
            .dot;
        println!("Expression: {}", expression);
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
