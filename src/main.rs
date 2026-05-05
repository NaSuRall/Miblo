pub mod cli;
pub mod engine;
pub mod writer;
pub mod generator;
pub mod runtime;
pub mod parser;
pub mod models;

fn main() {

    let cli = cli::run();

}
