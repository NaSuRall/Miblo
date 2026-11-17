mod cli;
mod config;
mod database;
mod generation;
mod runtime;
mod templates;
mod tui;
mod wizard;

fn main() -> std::process::ExitCode {
    match cli::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) if error.is::<wizard::Cancelled>() => {
            println!("Opération annulée. Aucun fichier modifié.");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
