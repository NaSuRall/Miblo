#[macro_use]
mod i18n;
mod cli;
mod config;
mod database;
mod generation;
mod runtime;
mod templates;
mod tui;
mod wizard;

fn main() -> std::process::ExitCode {
    let warning = i18n::initialize();
    if let Some(warning) = &warning {
        eprintln!("{warning}");
    }
    match cli::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) if error.is::<wizard::Cancelled>() => {
            println!(
                "{}",
                trf!(
                    "Operation cancelled. No files changed.",
                    "Opération annulée. Aucun fichier modifié."
                )
            );
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
