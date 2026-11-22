//! Background daemon loop wiring `sye-core`, `sye-platform`, and on-demand `sye-ui` subprocesses.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sye_core::chime::ChimeKind;
use sye_core::config::{BreakType, Config};
use sye_core::ipc::{IpcCommand, IpcResponse};
use sye_core::scheduler::{OverlayOutcome, Scheduler, SchedulerAction, SchedulerEvent};
use sye_core::session::SessionState;
use sye_core::traits::{IdleProvider, Notifier, OverlayLauncher};
use sye_platform::{
    AppPaths, IpcError, IpcServer, OsAudioPlayer, OsNotifier, SubprocessOverlayLauncher,
    SystemClock, SystemDndProvider, SystemIdleProvider, TrayEvent, TrayHandle,
};

const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Runs the single-instance background daemon until `Quit`.
pub(crate) fn run_daemon(paths: AppPaths, headless: bool) -> ExitCode {
    let server = match IpcServer::bind(&paths.socket_path) {
        Ok(srv) => srv,
        Err(IpcError::AlreadyRunning) => {
            println!("Safe Your Eyes is already running");
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("Failed to bind IPC socket: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = server.set_accept_nonblocking() {
        eprintln!("Failed to configure non-blocking IPC listener: {err}");
        return ExitCode::FAILURE;
    }

    let (config, load_err) = Config::load_or_default(&paths.config_file);
    if let Some(err) = load_err {
        eprintln!("Warning: falling back to default config ({err})");
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x5EED_CAFE, |d| d.as_nanos() as u64);

    let persist_state = config.persist_state;
    let mut scheduler = Scheduler::new(config.clone(), SystemClock::new(), seed);
    if persist_state
        && let Ok(raw) = std::fs::read_to_string(&paths.session_file)
        && let Ok(session) = SessionState::from_toml_str(&raw)
    {
        scheduler.restore_session(&session);
    }

    let notifier = OsNotifier::new();
    let audio = OsAudioPlayer::new();
    let idle_provider = SystemIdleProvider::new();
    let dnd_provider = SystemDndProvider::new();
    let mut overlay_launcher = if headless {
        None
    } else {
        SubprocessOverlayLauncher::current_exe().ok()
    };

    let (tray_tx, tray_rx) = mpsc::channel::<TrayEvent>();
    let mut tray = if headless {
        None
    } else {
        TrayHandle::try_new(&scheduler.tray_snapshot(), tray_tx).ok()
    };

    let mut headless_break_deadline: Option<Instant> = None;
    let mut last_policy_check = Instant::now();
    let mut running = true;

    let start_actions = scheduler.handle_event(SchedulerEvent::Start);
    execute_actions(
        start_actions,
        headless,
        &paths.session_file,
        &notifier,
        &audio,
        &mut overlay_launcher,
        &mut tray,
        &mut headless_break_deadline,
    );

    while running {
        loop {
            let mut should_quit = false;
            let mut open_settings = false;
            let mut deferred_actions: Vec<SchedulerAction> = Vec::new();

            let accepted = server.accept_once(Duration::from_millis(10), |cmd| match cmd {
                IpcCommand::Status => IpcResponse::Status(scheduler.status_snapshot()),
                IpcCommand::Enable => {
                    deferred_actions.extend(scheduler.handle_event(SchedulerEvent::Enable));
                    IpcResponse::Ok {
                        message: Some("Enabled".to_owned()),
                    }
                }
                IpcCommand::Disable { minutes } => {
                    let duration_secs = minutes.and_then(|m| {
                        if m == 0 {
                            None
                        } else {
                            Some(m.saturating_mul(60))
                        }
                    });
                    deferred_actions
                        .extend(scheduler.handle_event(SchedulerEvent::Disable { duration_secs }));
                    IpcResponse::Ok {
                        message: Some("Disabled".to_owned()),
                    }
                }
                IpcCommand::TakeBreak { kind } => {
                    deferred_actions
                        .extend(scheduler.handle_event(SchedulerEvent::TakeBreak { kind }));
                    IpcResponse::Ok {
                        message: Some("Break triggered".to_owned()),
                    }
                }
                IpcCommand::OpenSettings => {
                    open_settings = true;
                    IpcResponse::Ok {
                        message: Some("Opening settings".to_owned()),
                    }
                }
                IpcCommand::Quit => {
                    should_quit = true;
                    deferred_actions.extend(scheduler.handle_event(SchedulerEvent::Quit));
                    IpcResponse::Ok {
                        message: Some("Quitting".to_owned()),
                    }
                }
            });

            if !deferred_actions.is_empty() {
                execute_actions(
                    deferred_actions,
                    headless,
                    &paths.session_file,
                    &notifier,
                    &audio,
                    &mut overlay_launcher,
                    &mut tray,
                    &mut headless_break_deadline,
                );
            }

            if open_settings && !headless {
                spawn_settings_subprocess(&paths.config_file);
            }

            if should_quit {
                running = false;
                break;
            }

            match accepted {
                Ok(true) => continue,
                Ok(false) | Err(_) => break,
            }
        }

        if !running {
            break;
        }

        while let Ok(event) = tray_rx.try_recv() {
            let actions = match event {
                TrayEvent::Enable => scheduler.handle_event(SchedulerEvent::Enable),
                TrayEvent::Disable { minutes } => {
                    let duration_secs = minutes.and_then(|m| {
                        if m == 0 {
                            None
                        } else {
                            Some(m.saturating_mul(60))
                        }
                    });
                    scheduler.handle_event(SchedulerEvent::Disable { duration_secs })
                }
                TrayEvent::TakeBreak { kind } => {
                    scheduler.handle_event(SchedulerEvent::TakeBreak { kind })
                }
                TrayEvent::OpenSettings => {
                    if !headless {
                        spawn_settings_subprocess(&paths.config_file);
                    }
                    Vec::new()
                }
                TrayEvent::Quit => {
                    running = false;
                    scheduler.handle_event(SchedulerEvent::Quit)
                }
            };
            execute_actions(
                actions,
                headless,
                &paths.session_file,
                &notifier,
                &audio,
                &mut overlay_launcher,
                &mut tray,
                &mut headless_break_deadline,
            );
        }

        if !running {
            break;
        }

        if headless {
            if let Some(deadline) = headless_break_deadline
                && Instant::now() >= deadline
            {
                headless_break_deadline = None;
                let actions = scheduler
                    .handle_event(SchedulerEvent::OverlayFinished(OverlayOutcome::Completed));
                execute_actions(
                    actions,
                    headless,
                    &paths.session_file,
                    &notifier,
                    &audio,
                    &mut overlay_launcher,
                    &mut tray,
                    &mut headless_break_deadline,
                );
            }
        } else if let Some(launcher) = overlay_launcher.as_mut()
            && let Ok(Some(outcome)) = launcher.poll_outcome()
        {
            let actions = scheduler.handle_event(SchedulerEvent::OverlayFinished(outcome));
            execute_actions(
                actions,
                headless,
                &paths.session_file,
                &notifier,
                &audio,
                &mut overlay_launcher,
                &mut tray,
                &mut headless_break_deadline,
            );
        }

        if !headless && last_policy_check.elapsed() >= Duration::from_secs(1) {
            last_policy_check = Instant::now();
            if let Ok(idle) = idle_provider.idle_duration() {
                let actions = scheduler.handle_event(SchedulerEvent::IdleObserved(idle));
                execute_actions(
                    actions,
                    headless,
                    &paths.session_file,
                    &notifier,
                    &audio,
                    &mut overlay_launcher,
                    &mut tray,
                    &mut headless_break_deadline,
                );
            }
            let _ = &dnd_provider;
        }

        let tick_actions = scheduler.tick();
        if !tick_actions.is_empty() {
            execute_actions(
                tick_actions,
                headless,
                &paths.session_file,
                &notifier,
                &audio,
                &mut overlay_launcher,
                &mut tray,
                &mut headless_break_deadline,
            );
        }

        std::thread::sleep(POLL_INTERVAL);
    }

    if let Some(launcher) = overlay_launcher.as_mut() {
        let _ = launcher.close();
    }

    ExitCode::SUCCESS
}

#[expect(
    clippy::too_many_arguments,
    reason = "Action executor borrows host handles directly without allocating a wrapper struct"
)]
fn execute_actions(
    actions: Vec<SchedulerAction>,
    headless: bool,
    session_file: &Path,
    notifier: &OsNotifier,
    audio: &OsAudioPlayer,
    overlay_launcher: &mut Option<SubprocessOverlayLauncher>,
    tray: &mut Option<TrayHandle>,
    headless_break_deadline: &mut Option<Instant>,
) {
    for action in actions {
        match action {
            SchedulerAction::NotifyPreBreak {
                break_spec,
                warning_secs,
            } => {
                if !headless {
                    let _ = notifier.show_pre_break(
                        break_spec.break_type,
                        &break_spec.name,
                        warning_secs,
                    );
                }
            }
            SchedulerAction::DismissNotification => {
                if !headless {
                    let _ = notifier.dismiss();
                }
            }
            SchedulerAction::PlayChime { kind, volume } => {
                if !headless {
                    let _ = play_chime_safely(audio, kind, volume);
                }
            }
            SchedulerAction::ShowOverlay(req) => {
                if headless {
                    *headless_break_deadline =
                        Some(Instant::now() + Duration::from_secs(req.duration_secs));
                } else if let Some(launcher) = overlay_launcher.as_mut() {
                    let _ = launcher.launch(&req);
                }
            }
            SchedulerAction::CloseOverlay => {
                *headless_break_deadline = None;
                if let Some(launcher) = overlay_launcher.as_mut() {
                    let _ = launcher.close();
                }
            }
            SchedulerAction::UpdateTray(snapshot) => {
                if let Some(handle) = tray.as_mut() {
                    let _ = handle.update(&snapshot);
                }
            }
            SchedulerAction::PersistSession(session) => {
                let _ = session.save_to_path(session_file);
            }
        }
    }
}

fn play_chime_safely(audio: &OsAudioPlayer, kind: ChimeKind, volume: u8) -> Result<(), String> {
    audio.play_chime(kind, volume).map_err(|e| e.to_string())
}

fn spawn_settings_subprocess(config_file: &Path) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = Command::new(exe)
        .arg("--internal-settings")
        .arg("--config")
        .arg(config_file)
        .spawn();
}

/// Formats a human-readable `--status` line for CLI users.
pub(crate) fn format_human_status(snapshot: &sye_core::scheduler::StatusSnapshot) -> String {
    let enabled_label = if snapshot.enabled {
        "enabled"
    } else {
        "disabled"
    };
    let next_label = match (&snapshot.next_break_type, &snapshot.next_break_name) {
        (Some(kind), Some(name)) => {
            let tier = match kind {
                BreakType::Short => "short",
                BreakType::Long => "long",
            };
            format!("{tier} ({name}) in {}s", snapshot.seconds_until_next_break)
        }
        _ => "none".to_owned(),
    };
    format!(
        "Safe Your Eyes: {:?} ({enabled_label}) | Next break: {next_label} | Consecutive skips: {}",
        snapshot.state, snapshot.consecutive_skips
    )
}
