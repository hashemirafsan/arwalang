mod annotations;
mod cli;
mod codegen;
mod di;
mod errors;
mod ir;
mod lexer;
mod lifecycle;
mod modules;
mod parser;
mod resolver;
mod routes;
mod typechecker;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}
