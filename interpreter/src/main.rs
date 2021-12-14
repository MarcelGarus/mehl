mod compiler;
mod utils;
mod vm;

use crate::compiler::*;
use crate::vm::{Value, Vm, VmOperation, VmStatus};
use colored::Colorize;
use itertools::Itertools;
use log::debug;
use lspower::jsonrpc::Result;
use lspower::lsp::*;
use lspower::{Client, LanguageServer, LspService, Server};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use std::collections::HashMap;
use std::io::{stdout, Write};
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
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let options = Mehl::from_args();
    debug!("{:#?}", options);
    match options {
        Mehl::Run => {
            debug!("Running test.mehl.\n");

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
            debug!("AST: {}\n", &ast);

            let mut hir = ast.compile_to_hir();
            debug!("Unoptimized HIR: {}", hir);
            hir.optimize();
            debug!("HIR: {}", hir);

            let mut lir = hir.compile_to_lir();
            debug!("Unoptimized LIR: {}", lir);
            lir.optimize();
            debug!("LIR: {}", lir);

            debug!("Compiling to byte code...");
            let byte_code = lir.compile_to_byte_code();
            debug!("Byte code: {:?}", byte_code);

            debug!("Running in VM...");
            let mut ambients = HashMap::new();
            ambients.insert("stdout".into(), Value::ChannelSendEnd(0));
            ambients.insert("stdin".into(), Value::ChannelReceiveEnd(1));
            let mut vm = Vm::new(byte_code, ambients);
            loop {
                vm.run(30);
                let operations = vm
                    .pending_operations()
                    .into_iter()
                    .map(|it| (*it).clone())
                    .collect_vec();
                debug!("Vm: {:?}", vm);
                for operation in operations {
                    match operation {
                        VmOperation::Send(channel_id, message) => match channel_id {
                            0 => {
                                let mut out = stdout();
                                out.write(
                                    if let Value::String(string) = &message {
                                        string.clone()
                                    } else {
                                        message.to_string()
                                    }
                                    .as_bytes(),
                                )
                                .unwrap();
                                out.flush().unwrap();
                                vm.resolve_send(channel_id, message);
                            }
                            _ => panic!("Unknown channel id {}.", channel_id),
                        },
                        VmOperation::Receive(channel_id) => match channel_id {
                            1 => {
                                let mut input = String::new();
                                std::io::stdin()
                                    .read_line(&mut input)
                                    .expect("Couldn't read line.");
                                vm.resolve_receive(channel_id, Value::String(input));
                            }
                            _ => panic!("Unknown channel id {}.", channel_id),
                        },
                    }
                }
                match vm.status() {
                    VmStatus::Running => {}
                    VmStatus::Done(value) => {
                        println!(
                            "{}",
                            format!("Done running: {}", value).bright_green().bold()
                        );
                        break;
                    }
                    VmStatus::Panicked(value) => {
                        println!("{}", format!("Panicked: {}", value).bright_red().bold());
                        break;
                    }
                    VmStatus::WaitingForPendingOperations => {
                        panic!("Operations should have been handled above.")
                    }
                }
            }
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
