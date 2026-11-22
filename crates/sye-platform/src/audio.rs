//! Cross-platform non-blocking WAV chime playback (`sye_core::AudioPlayer`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use sye_core::{AudioPlayer, ChimeKind, PlatformError, synthesize_chime_wav};

static AUDIO_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
struct PlayerSpec {
    program: String,
    args: Vec<String>,
}

/// Non-blocking OS audio player for synthesized WAV chimes.
///
/// Writes WAV bytes to a unique temporary file, spawns a platform player (`pw-play`/`paplay`/`aplay`
/// on Linux, `afplay` on macOS, PowerShell `Media.SoundPlayer` on Windows), and reaps the child +
/// deletes the temporary file on a background thread.
#[derive(Debug, Clone)]
pub struct OsAudioPlayer {
    temp_dir: PathBuf,
    players: Vec<PlayerSpec>,
}

impl OsAudioPlayer {
    /// Creates an `OsAudioPlayer` using the OS runtime/temp directory and default platform players.
    pub fn new() -> Self {
        Self::with_temp_dir(default_audio_temp_dir())
    }

    /// Creates an `OsAudioPlayer` writing temporary WAV files under `temp_dir`.
    pub fn with_temp_dir(temp_dir: PathBuf) -> Self {
        Self {
            temp_dir,
            players: default_player_specs(),
        }
    }

    #[cfg(test)]
    fn with_players(temp_dir: PathBuf, players: Vec<PlayerSpec>) -> Self {
        Self { temp_dir, players }
    }

    /// Synthesizes and plays a pre-break or post-break chime at `volume` (clamped to `0..=100`).
    ///
    /// A volume of `0` returns `Ok(())` without writing a file or spawning a process.
    pub fn play_chime(&self, kind: ChimeKind, volume: u8) -> Result<(), PlatformError> {
        let clamped = volume.min(100);
        if clamped == 0 {
            return Ok(());
        }
        let wav = synthesize_chime_wav(kind, clamped);
        self.play_wav(&wav)
    }
}

impl Default for OsAudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer for OsAudioPlayer {
    fn play_wav(&self, wav_bytes: &[u8]) -> Result<(), PlatformError> {
        if wav_bytes.is_empty() {
            return Ok(());
        }

        fs::create_dir_all(&self.temp_dir).map_err(|err| PlatformError::Io(err.to_string()))?;

        let seq = AUDIO_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = self
            .temp_dir
            .join(format!("sye-chime-{}-{seq}.wav", std::process::id()));
        fs::write(&path, wav_bytes).map_err(|err| PlatformError::Io(err.to_string()))?;

        match spawn_first_available_player(&self.players, &path) {
            Ok(mut child) => {
                let cleanup_path = path.clone();
                if thread::Builder::new()
                    .name("sye-audio-reaper".to_owned())
                    .spawn(move || {
                        let _ = child.wait();
                        let _ = fs::remove_file(&cleanup_path);
                    })
                    .is_err()
                {
                    let _ = fs::remove_file(&path);
                }
                Ok(())
            }
            Err(err) => {
                let _ = fs::remove_file(&path);
                Err(err)
            }
        }
    }
}

fn default_audio_temp_dir() -> PathBuf {
    #[cfg(unix)]
    {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR")
            && !runtime_dir.trim().is_empty()
        {
            return PathBuf::from(runtime_dir).join("safe-your-eyes");
        }
    }
    std::env::temp_dir().join("safe-your-eyes")
}

fn default_player_specs() -> Vec<PlayerSpec> {
    #[cfg(target_os = "macos")]
    {
        vec![PlayerSpec {
            program: "afplay".to_owned(),
            args: Vec::new(),
        }]
    }
    #[cfg(target_os = "windows")]
    {
        vec![PlayerSpec {
            program: "powershell.exe".to_owned(),
            args: vec![
                "-NoProfile".to_owned(),
                "-NonInteractive".to_owned(),
                "-WindowStyle".to_owned(),
                "Hidden".to_owned(),
                "-Command".to_owned(),
            ],
        }]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        vec![
            PlayerSpec {
                program: "pw-play".to_owned(),
                args: Vec::new(),
            },
            PlayerSpec {
                program: "paplay".to_owned(),
                args: Vec::new(),
            },
            PlayerSpec {
                program: "aplay".to_owned(),
                args: vec!["-q".to_owned()],
            },
        ]
    }
}

fn spawn_first_available_player(
    players: &[PlayerSpec],
    wav_path: &Path,
) -> Result<Child, PlatformError> {
    let mut last_error = String::from("no audio playback backend configured");

    for spec in players {
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args);

        #[cfg(target_os = "windows")]
        if spec.program.eq_ignore_ascii_case("powershell.exe") {
            let escaped = wav_path.to_string_lossy().replace('\'', "''");
            cmd.arg(format!(
                "(New-Object Media.SoundPlayer '{escaped}').PlaySync()"
            ));
        } else {
            cmd.arg(wav_path);
        }

        #[cfg(not(target_os = "windows"))]
        {
            cmd.arg(wav_path);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        match cmd.spawn() {
            Ok(child) => return Ok(child),
            Err(err) => {
                last_error = format!("{}: {err}", spec.program);
            }
        }
    }

    Err(PlatformError::Unavailable(format!(
        "no OS audio player available ({last_error})"
    )))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::test_util::TestTempDir;

    #[test]
    fn empty_wav_and_zero_volume_create_no_files() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TestTempDir::new("audio-empty")?;
        let player = OsAudioPlayer::with_temp_dir(tmp.path().to_path_buf());

        player.play_wav(&[])?;
        player.play_chime(ChimeKind::PreBreak, 0)?;

        let entries: Vec<_> = fs::read_dir(tmp.path())?.collect();
        assert!(entries.is_empty());
        Ok(())
    }

    #[test]
    fn missing_player_returns_unavailable_and_cleans_temp_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = TestTempDir::new("audio-missing")?;
        let player = OsAudioPlayer::with_players(
            tmp.path().to_path_buf(),
            vec![PlayerSpec {
                program: "sye-nonexistent-audio-player-binary".to_owned(),
                args: Vec::new(),
            }],
        );

        let result = player.play_chime(ChimeKind::PreBreak, 80);
        assert!(matches!(result, Err(PlatformError::Unavailable(_))));

        let entries: Vec<_> = fs::read_dir(tmp.path())?.collect();
        assert!(entries.is_empty(), "temp WAV file must be removed on error");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn successful_spawn_cleans_up_temp_wav_asynchronously() -> Result<(), Box<dyn std::error::Error>>
    {
        let tmp = TestTempDir::new("audio-ok")?;
        let player = OsAudioPlayer::with_players(
            tmp.path().to_path_buf(),
            vec![PlayerSpec {
                program: "/bin/true".to_owned(),
                args: Vec::new(),
            }],
        );

        // Volume > 100 must clamp cleanly without error.
        player.play_chime(ChimeKind::PostBreak, 250)?;

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let remaining = fs::read_dir(tmp.path())?.count();
            if remaining == 0 {
                break;
            }
            if Instant::now() >= deadline {
                return Err("timed out waiting for reaper thread to delete temp WAV".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}
