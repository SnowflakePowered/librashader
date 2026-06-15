use clap::{Parser, Subcommand};
use std::env;
use std::process::{Command, ExitCode};

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run librashader-build-script to build the librashader C API.
    #[command(disable_help_flag = true)]
    Build {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run librashader-cli to reflect and debug 'slang' shaders and presets.
    #[command(disable_help_flag = true)]
    Cli {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn run(package: &str, args: &[String]) -> ExitCode {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(&cargo);
    cmd.args(["run", "--package", package, "--"]);
    cmd.args(args);

    match cmd.status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(_) => ExitCode::FAILURE,
    }
}

pub fn main() -> ExitCode {
    let args = Args::parse();

    match args.command {
        Commands::Build { args } => run("librashader-build-script", &args),
        Commands::Cli { args } => run("librashader-cli", &args),
    }
}
