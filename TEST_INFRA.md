# Test Infrastructure: `safe-your-eyes` Opaque-Box E2E Suite

## 1. Test Philosophy

The `e2e/` test suite is an **opaque-box, requirement-driven** end-to-end verification suite derived strictly from `PROJECT.md` (`§ Feature Inventory` F1–F21 and `§ Interface Contracts § 1–2`).

- **Zero Rust internals coupling**: Tests never import Rust crates, read `crates/` source code, or inspect internal structs. Every assertion observes only external process behavior: CLI flags, exit codes, `stdout`/`stderr`, TOML configs (`--config`), Unix domain sockets / pipes (`--socket`), and `--status --json` responses.
- **Zero external Python dependencies**: Uses **Python 3.11+ standard library only** (`argparse`, `atexit`, `concurrent.futures`, `json`, `os`, `pathlib`, `re`, `secrets`, `shutil`, `signal`, `socket`, `subprocess`, `sys`, `threading`, `time`, `tomllib`, `traceback`).
- **Deterministic & fast**: Timing-sensitive scheduler tests use compressed intervals (2–3s intervals, 1s durations, 1s pre-break warnings) and monotonic polling (`wait_until`) instead of brittle fixed sleeps. All 4 tiers run end-to-end in ~60–90 seconds.
- **Strict workspace & process hygiene**: All temporary configs, daemon logs, isolated `HOME`/`XDG_*`/`TMPDIR` directories, and IPC sockets live strictly under `<repo>/.tmp/e2e/` (never OS temp directories). Signal handlers, `atexit`, and 3-stage daemon teardown guarantee zero leaked processes or artifacts on pass, fail, or interrupt.

---

## 2. Feature Inventory Mapped to `PROJECT.md` (F1–F21)

| # | Feature | Milestone | E2E Coverage | Covered in `e2e/` (Opaque-Box) | Covered in Unit / Integration / GUI / CI |
|---|---|---|---|---|---|
| **F1** | Project Specs, ADRs, Glossary & Hygiene | M1 | Non-E2E | — | Repo docs, `.gitignore`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`. |
| **F2** | Dual-Tier Break Queue | M1 | **Full E2E** | Default 13 short + 6 long exercises via `--print-default-config` (T1); compressed short→short→long cycle (T1, T3, T4); `random_order = false` sequential wrap-around (T1, T3); per-break override config (T2). | `sye-core` unit tests for exact 4:1 math, random shuffle distribution, empty-queue guards. |
| **F3** | 7-State Scheduler State Machine | M1 | **Full E2E** (5/7 states) | `Waiting → PreBreak → Break → Waiting`, `Stopped`, `Quit` via `--status --json` & `--headless` (T1–T4). | `sye-core` unit tests with mock `Clock` for `Start`, `Resting`, and reentrancy guards. |
| **F4** | Strict Break, Postpone, Skip & Anti-Accidental Lock | M1 | **Partial E2E** | Config fields (`strict_break`, `allow_postpone`, `postpone_duration_secs`, `shortcut_disable_secs`) validation & `strict_break = true` headless lifecycle (T1, T3). | `sye-core` & `sye-ui` tests for interactive Skip/Postpone actions, queue preservation on postpone, 2s shortcut lock. |
| **F5** | Manual Break & Timed Pause/Disable | M1 | **Full E2E** | `-t`, `-b`, `-l`, `-b -l` conflict guard, `--disable [MIN]`, `--disable 0`, `--enable`, short aliases `-d`/`-e`/`-q` (T1–T4). | `sye-core` unit tests for exact minute auto-resume with mock `Clock`. |
| **F6** | Natural Rest / Suspend Cycle Reset | M1 | Non-E2E | — | `sye-core` unit tests (`is_resting` + `skip_long_break()` via mock `Clock`). |
| **F7** | Smart Pause Policy | M1 | **Partial E2E** | `[smart_pause]` schema validation, startup with `enabled = true`, headless graceful degrade (T1, T3). | `sye-core` unit tests with mock `IdleProvider` for idle credit-back, full reset, `postpone_if_active`. |
| **F8** | Do Not Disturb Policy | M1 | **Partial E2E** | `[do_not_disturb]` schema validation, startup with `enabled = true`, headless graceful degrade (T1, T3). | `sye-core` unit tests with mock `DndProvider` for fullscreen, window precedence, battery check. |
| **F9** | Limit Consecutive Skipping Policy | M1 | **Partial E2E** | `[limit_consecutive_skips]` schema validation, `consecutive_skips` field in `--status --json` (T1, T3). | `sye-core` unit tests for skip/postpone counter increment, limit enforcement, reset on break completion. |
| **F10** | TOML Config, Validation & Session Persistence | M1 | **Full E2E** | `--print-default-config`, `--validate-config` (valid/invalid/missing/malformed), corrupted-config fallback, `persist_state = true/false` across restart (T1–T4). | `sye-core` unit tests for schema migration and fallback internals. |
| **F11** | Pure-Rust WAV Chime Synthesizer & IPC Protocol | M1 | **Partial E2E** | JSON-line IPC (`Status`, `Enable`, `Disable`, `TakeBreak`, `Quit`) across all tiers; `[audible_alert]` config validation & headless fallback (T1, T3). | `sye-core` unit tests for PCM WAV header & sine-wave chime synthesis. |
| **F12** | Cross-Platform System Tray | M2 | **Partial E2E** | Graceful headless degradation (`--headless`, T1–T4). | Platform tests for `tray-icon`/`ksni` menu items, tooltips, and icons. |
| **F13** | Cross-Platform Idle & DND Providers | M2 | **Partial E2E** | Graceful fallback when OS idle/DND providers are unavailable in headless CI (T3). | `sye-platform` tests for X11/Wayland/macOS/Windows idle, fullscreen, battery probes. |
| **F14** | Cross-Platform Notifications & Audio Playback | M2 | **Partial E2E** | Graceful fallback with `audible_alert` enabled in headless CI without audio/D-Bus (T3). | `sye-platform` tests for volume clamp & OS command/notifier adapters. |
| **F15** | Single-Instance Local IPC & Autostart | M2 | **Full E2E** (IPC) | Single-instance guard (`"Safe Your Eyes is already running"`), `--socket` isolation, concurrent clients, `SIGKILL` & dead-UDS stale-socket recovery (T1–T4). | `sye-platform` unit tests for OS autostart entry generation. |
| **F16** | Multi-Monitor Fullscreen Break Overlay UI | M3 | Non-E2E (GUI) | — (`--headless` skips GUI overlay subprocesses). | `sye-ui` unit & display-server verification for fade-in, countdown, `Esc`/`Space`, multi-monitor layout. |
| **F17** | Compact Settings UI | M3 | Non-E2E (GUI) | CLI help presence check (T1). | `sye-ui` unit & display-server verification for Settings window Save/Cancel/Reset. |
| **F18** | Single-Binary CLI, Daemon & Subprocess Orchestration | M4 | **Full E2E** | All CLI flags, exit codes, exact stdout/stderr contracts, daemon lifecycle, `SIGTERM`/`SIGKILL` handling (T1–T4). | Release binary size check & subprocess launcher unit tests. |
| **F19** | Cross-Platform CI Matrix | M4 | Non-E2E | — | `.github/workflows/ci.yml` on Linux, macOS, Windows. |
| **F20** | Opaque-Box E2E Test Suite (Tiers 1–4) | E2E Track | **This Deliverable** | `TEST_INFRA.md`, `e2e/run_e2e.py`, `e2e/harness.py`, `e2e/tests/test_tier{1..4}_*.py`. | — |
| **F21** | Final Verification & Tier 5 Hardening | M5 | **Gate + Hardening** | 100% pass of Tiers 1–4 required before M5 Phase 2. | M5 Phase 2 adds Tier 5 white-box adversarial tests. |

---

## 3. Test Architecture

```
e2e/
├── run_e2e.py                        # Executable CLI runner, tier orchestration, summary & exit codes
├── harness.py                        # Stdlib-only harness: SyeTestEnv, DaemonHandle, TOML & polling helpers
└── tests/
    ├── __init__.py
    ├── test_tier1_features.py        # Tier 1: Core Feature Coverage (28 tests across 5 areas)
    ├── test_tier2_boundaries.py      # Tier 2: Boundary & Corner Cases (30 tests across 5 areas)
    ├── test_tier3_combinations.py    # Tier 3: Pairwise Cross-Feature Combinations (8 tests)
    └── test_tier4_scenarios.py       # Tier 4: Real-World End-User Scenarios (6 scenarios)
```

### 3.1 Workspace Isolation (`<repo>/.tmp/e2e/`)
- Each test runs inside `SyeTestEnv`, which creates `<repo>/.tmp/e2e/run_<pid>/<test_id>/` containing dedicated `home/`, `tmp/`, `xdg_config/`, `xdg_data/`, `xdg_state/`, `xdg_cache/`, and `xdg_runtime/` (`0o700`).
- Every subprocess receives a sanitized environment with `HOME`, `TMPDIR`, `TEMP`, `TMP`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`, and `XDG_RUNTIME_DIR` pointing inside that test's isolated directory.
- Config builders bootstrap from `safe-your-eyes --print-default-config` (parsed via stdlib `tomllib`), deep-merge only test overrides, and serialize via `toml_dumps`. This preserves schema version fields and any forward-compatible defaults.

### 3.2 Flat UDS Socket Path Strategy
- Linux (`sun_path = 108` bytes) and macOS (`sun_path = 104` bytes) enforce strict Unix domain socket path limits.
- Instead of nesting sockets under per-test directories, `harness.py` allocates short flat socket paths directly under `<repo>/.tmp/e2e/s_<pid4><seq3>.sock` and asserts `len(utf8_bytes) <= 103` before launching any daemon.
- On Windows (`os.name == "nt"`), named pipe identifiers (`sye_e2e_<pid4><seq3>`) are allocated instead.

### 3.3 Daemon Lifecycle & 3-Stage Cleanup
- `DaemonHandle` spawns daemons with `start_new_session=True` (dedicated process group) and **file-backed** `stdout`/`stderr` logs inside the test directory (eliminating 64 KB pipe-buffer deadlocks).
- `stop()` executes a 3-stage guaranteed teardown:
  1. **Graceful IPC**: `<bin> --socket <sock> --quit` + `proc.wait(timeout)`.
  2. **`SIGTERM`**: `os.killpg(pid, SIGTERM)` + wait.
  3. **`SIGKILL`**: `os.killpg(pid, SIGKILL)` + wait.
- Global `atexit`, `SIGINT`, and `SIGTERM` hooks stop any tracked daemons and delete `<repo>/.tmp/e2e/run_<pid>/` and allocated sockets (unless `--keep-tmp` is passed).

---

## 4. Tier 4 Real-World Application Scenarios Catalog

| ID | Scenario | Summary |
|---|---|---|
| `T4_01` | **Full Workday Short → Short → Long Cycle** | Continuous work session verifying natural timer transitions (`Waiting → PreBreak → Break → Waiting`), exercise rotation (`Short 1 → Short 2 → Long 1 → Short 3`), cycle reset, and clean `--quit`. |
| `T4_02` | **Meeting Pause & Resume Workflow** | User pauses for a 30-minute call (`--disable 30`), inspects JSON and human-readable `--status`, confirms no breaks fire while paused, resumes early (`--enable`), takes a manual break (`-b`), and quits cleanly. |
| `T4_03` | **Ad-Hoc Manual Break Triggers & Conflict Guard** | User triggers `-b`, `-l`, and `-t` sequentially, hits the `-b -l` conflict guard (exit `1` without disrupting the running daemon), quits (`--quit`), and verifies post-quit `--status` reports not running. |
| `T4_04` | **Config Customization, Validation, Bad-Edit Rejection & Recovery** | Power user exports defaults via `--print-default-config`, writes custom exercises, validates and runs, catches syntax and semantic errors via `--validate-config`, fixes config, restarts, and verifies recovery. |
| `T4_05` | **Crash (`SIGKILL`) & Stale-Socket Recovery** | Daemon is abruptly killed with `SIGKILL` leaving a dead UDS file on disk; `--status` reports not running; relaunching on the same `--socket` recovers cleanly and restores full IPC. |
| `T4_06` | **Multi-Client Status-Bar Polling + Control Commands** | Two background status-bar poller threads continuously query `--status --json` while the user executes `--disable 10`, `--enable`, `-b`, a duplicate daemon launch, and `--quit`, verifying zero poll failures and full state observability. |

---

## 5. Coverage Thresholds & Gate Policy

| Tier | Requirement | Implemented |
|---|---|---|
| **Tier 1** (`test_tier1_features.py`) | >= 5 tests per major feature area (5 areas) | **28 tests** (6 + 5 + 6 + 6 + 5) |
| **Tier 2** (`test_tier2_boundaries.py`) | >= 5 boundary/corner tests per feature area (5 areas) | **30 tests** (6 + 6 + 6 + 6 + 6) |
| **Tier 3** (`test_tier3_combinations.py`) | Pairwise cross-feature combinations | **8 tests** |
| **Tier 4** (`test_tier4_scenarios.py`) | >= 5 realistic end-user scenarios | **6 scenarios** |
| **Total** | **100% pass required** (`exit 0` only when `failed == 0` and `errors == 0`) | **72 tests** |

---

## 6. How to Run

```bash
# Run all 4 tiers against the debug binary (default --bin target/debug/safe-your-eyes)
python3 e2e/run_e2e.py --bin target/debug/safe-your-eyes --tier all

# Run a single tier or comma-separated tiers with verbose output
python3 e2e/run_e2e.py --bin target/debug/safe-your-eyes --tier 1 -v
python3 e2e/run_e2e.py --bin target/debug/safe-your-eyes --tier 1,2

# Run against the release binary
python3 e2e/run_e2e.py --bin target/release/safe-your-eyes --tier all

# Retain <repo>/.tmp/e2e/run_<pid>/ logs and configs for post-mortem debugging
python3 e2e/run_e2e.py --bin target/debug/safe-your-eyes --tier 4 --keep-tmp
```

### Runner Exit Codes
- `0`: All selected tests passed (100% pass).
- `1`: One or more tests failed or errored.
- `2`: Invalid CLI arguments or missing/non-executable `--bin` path.
