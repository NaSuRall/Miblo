use colored::Colorize;

pub mod cli;
pub mod engine;
pub mod generator;
pub mod models;
pub mod parser;
pub mod runtime;
pub mod writer;
pub mod services;

fn main() {
    let cli = cli::run();

    match cli {
        Ok(_) => println!("{}", "Your API has been successfully generated".cyan()),
        Err(e) => println!("error : {e:?}"),
    };
}
