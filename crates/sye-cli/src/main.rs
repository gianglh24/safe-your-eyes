//! Single-binary entry point for `safe-your-eyes`.

mod cli;
mod daemon;

use std::process::ExitCode;

use sye_core::config::{BreakType, Config};
use sye_core::ipc::{IpcCommand, IpcResponse};
use sye_core::scheduler::OverlayRequest;
use sye_platform::{AppPaths, IpcClient, IpcError};

use crate::cli::{CommandMode, HELP_TEXT, parse_args};

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    let paths = match AppPaths::with_overrides(args.config, args.socket) {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("Failed to resolve application paths: {err}");
            return ExitCode::FAILURE;
        }
    };

    match args.mode {
        CommandMode::Help => {
            print!("{HELP_TEXT}");
            ExitCode::SUCCESS
        }
        CommandMode::Version => {
            println!("safe-your-eyes {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        CommandMode::PrintDefaultConfig => match Config::default().to_toml_string() {
            Ok(toml_str) => {
                print!("{toml_str}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("Failed to serialize default config: {err}");
                ExitCode::FAILURE
            }
        },
        CommandMode::ValidateConfig => match Config::load_from_path(&paths.config_file) {
            Ok(_) => {
                println!("Configuration is valid: {}", paths.config_file.display());
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!(
                    "Invalid configuration ({}): {err}",
                    paths.config_file.display()
                );
                ExitCode::FAILURE
            }
        },
        CommandMode::InternalOverlay { payload_json } => {
            let request: OverlayRequest = match serde_json::from_str(&payload_json) {
                Ok(req) => req,
                Err(err) => {
                    eprintln!("Invalid overlay payload JSON: {err}");
                    return ExitCode::FAILURE;
                }
            };
            match sye_ui::run_overlay(request) {
                Ok(outcome) => match serde_json::to_string(&outcome) {
                    Ok(line) => {
                        println!("{line}");
                        ExitCode::SUCCESS
                    }
                    Err(err) => {
                        eprintln!("Failed to serialize overlay outcome: {err}");
                        ExitCode::FAILURE
                    }
                },
                Err(err) => {
                    eprintln!("{err}");
                    ExitCode::FAILURE
                }
            }
        }
        CommandMode::InternalSettings => match sye_ui::run_settings(paths.config_file) {
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::FAILURE
            }
        },
        CommandMode::Daemon => daemon::run_daemon(paths, args.headless),
        CommandMode::Status { json } => send_and_print_status(&paths, json),
        CommandMode::Enable => send_simple_command(&paths, IpcCommand::Enable),
        CommandMode::Disable { minutes } => {
            send_simple_command(&paths, IpcCommand::Disable { minutes })
        }
        CommandMode::TakeBreak => send_simple_command(&paths, IpcCommand::TakeBreak { kind: None }),
        CommandMode::ShortBreak => send_simple_command(
            &paths,
            IpcCommand::TakeBreak {
                kind: Some(BreakType::Short),
            },
        ),
        CommandMode::LongBreak => send_simple_command(
            &paths,
            IpcCommand::TakeBreak {
                kind: Some(BreakType::Long),
            },
        ),
        CommandMode::Settings => {
            match IpcClient::send_command(&paths.socket_path, &IpcCommand::OpenSettings) {
                Ok(IpcResponse::Ok { .. }) => ExitCode::SUCCESS,
                Ok(IpcResponse::Error { message, .. }) => {
                    eprintln!("{message}");
                    ExitCode::FAILURE
                }
                Ok(IpcResponse::Status(_)) => ExitCode::SUCCESS,
                Err(IpcError::NotRunning) => match sye_ui::run_settings(paths.config_file) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(err) => {
                        eprintln!("{err}");
                        ExitCode::FAILURE
                    }
                },
                Err(err) => {
                    eprintln!("{err}");
                    ExitCode::FAILURE
                }
            }
        }
        CommandMode::Quit => send_simple_command(&paths, IpcCommand::Quit),
    }
}

fn send_and_print_status(paths: &AppPaths, json: bool) -> ExitCode {
    match IpcClient::send_command(&paths.socket_path, &IpcCommand::Status) {
        Ok(IpcResponse::Status(snapshot)) => {
            if json {
                match serde_json::to_string(&snapshot) {
                    Ok(line) => {
                        println!("{line}");
                        ExitCode::SUCCESS
                    }
                    Err(err) => {
                        eprintln!("Failed to serialize status JSON: {err}");
                        ExitCode::FAILURE
                    }
                }
            } else {
                println!("{}", daemon::format_human_status(&snapshot));
                ExitCode::SUCCESS
            }
        }
        Ok(IpcResponse::Error { message, .. }) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
        Ok(IpcResponse::Ok { message }) => {
            if let Some(msg) = message {
                println!("{msg}");
            }
            ExitCode::SUCCESS
        }
        Err(IpcError::NotRunning) => {
            eprintln!("Safe Your Eyes is not running");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn send_simple_command(paths: &AppPaths, cmd: IpcCommand) -> ExitCode {
    match IpcClient::send_command(&paths.socket_path, &cmd) {
        Ok(IpcResponse::Ok { message }) => {
            if let Some(msg) = message {
                println!("{msg}");
            }
            ExitCode::SUCCESS
        }
        Ok(IpcResponse::Status(snapshot)) => {
            println!("{}", daemon::format_human_status(&snapshot));
            ExitCode::SUCCESS
        }
        Ok(IpcResponse::Error { message, .. }) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
        Err(IpcError::NotRunning) => {
            eprintln!("Safe Your Eyes is not running");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
