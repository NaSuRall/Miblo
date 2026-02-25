use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Commands {
    CreateApp { name: String },
}

#[derive(Parser)]
#[command(name = "miblo", about = "CLI pour générer des API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::CreateApp { name } => {
            println!("Création de l'application {}", name);
        }
    }
}
