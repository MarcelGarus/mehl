mod compiler;
mod runner;
mod utils;
mod vm;

use crate::compiler::*;
use crate::vm::{Fiber, FiberStatus, Value};
use colored::Colorize;
use lspower::jsonrpc::Result;
use lspower::lsp::*;
use lspower::{Client, LanguageServer, LspService, Server};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::collections::HashMap;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "mehl", about = "The Mehl CLI.")]
enum Mehl {
    /// Runs a Mehl file.
    Run,

    /// Starts the LSP.
    Lsp,
}

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Warn,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let options = Mehl::from_args();
    println!("{:#?}", options);
    match options {
        Mehl::Run => {
            println!("Running test.mehl.");

            let code = {
                let core_code =
                    std::fs::read_to_string("core.mehl").expect("File core.mehl not found.");
                let test_code =
                    std::fs::read_to_string("test.mehl").expect("File test.mehl not found.");
                format!("{}\n{}", core_code, test_code)
            };

            let ast = match code.parse_to_asts() {
                Ok(it) => it,
                Err(err) => panic!("Couldn't parse ASTs of core.mehl: {}", err),
            };
            println!("AST: {}", &ast);

            let mut hir = ast.compile_to_hir();
            hir.optimize();
            println!("HIR: {}", hir);

            let lir = hir.compile_to_lir();
            println!("LIR: {}", lir);

            println!("Compiling to byte code...");
            let byte_code = lir.compile_to_byte_code();
            println!("Byte code: {:?}", byte_code);

            println!("Running in VM...");
            let mut ambients = HashMap::new();
            ambients.insert("stdout".into(), Value::ChannelSendEnd(0));
            ambients.insert("stdin".into(), Value::ChannelReceiveEnd(1));
            let mut fiber = Fiber::new(byte_code, ambients);
            loop {
                fiber.run(30);
                match fiber.status() {
                    FiberStatus::Running => {}
                    FiberStatus::Done(value) => {
                        println!("{}", format!("Done running: {:?}", value).green());
                        break;
                    }
                    FiberStatus::Sending(channel_id, message) => match channel_id {
                        0 => {
                            println!("{}", format!("🌮> {:?}", message).yellow());
                            fiber.resolve_sending();
                        }
                        _ => panic!("Unknown channel id {}.", channel_id),
                    },
                    FiberStatus::Receiving(channel_id) => match channel_id {
                        1 => {
                            let mut input = String::new();
                            std::io::stdin()
                                .read_line(&mut input)
                                .expect("Couldn't read line.");
                            fiber.resolve_receiving(Value::String(input));
                        }
                        _ => panic!("Unknown channel id {}.", channel_id),
                    },
                }
            }
            println!("{:?}", fiber);

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
        Mehl::Lsp => {
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
