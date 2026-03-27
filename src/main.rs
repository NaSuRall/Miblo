mod cli;
mod generator;
mod parser;
mod runtime;
pub mod writer;

fn main() {
    let _ = cli::lunch();
}
