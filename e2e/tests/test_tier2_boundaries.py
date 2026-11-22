"""Tier 2: Boundary & corner case opaque-box E2E tests (5 areas, 30 tests)."""

from __future__ import annotations

import concurrent.futures
import json
import os
import socket
import time

from e2e.harness import SyeTestEnv, normalize_state

CONFLICT_ERR = "Cannot combine -b/--short-break with -l/--long-break"
NOT_RUNNING_ERR = "Safe Your Eyes is not running"

# ---------------------------------------------------------------------------
# Area 2A: CLI Argument & Flag Boundary Cases (6 tests)
# ---------------------------------------------------------------------------


def test_t2_cli_01_conflicting_short_and_long_break_flags(env: SyeTestEnv) -> None:
    """-b -l fails with contract error both without and with a running daemon."""
    res = env.run_cli(["-b", "-l"], socket=env.default_socket)
    assert res.returncode == 1, f"Expected rc=1 without daemon, got {res.returncode}"
    assert CONFLICT_ERR in res.stderr, f"Missing conflict msg in stderr: {res.stderr!r}"

    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    env.start_daemon(config=cfg)
    res2 = env.run_cli(["-b", "-l"], socket=env.default_socket)
    assert res2.returncode == 1, f"Expected rc=1 with daemon, got {res2.returncode}"
    assert CONFLICT_ERR in res2.stderr, f"Missing conflict msg in stderr: {res2.stderr!r}"


def test_t2_cli_02_conflicting_long_and_mixed_break_flags(env: SyeTestEnv) -> None:
    """--short-break --long-break and mixed -b --long-break fail with contract error."""
    for flags in (
        ["--short-break", "--long-break"],
        ["-b", "--long-break"],
        ["--short-break", "-l"],
    ):
        res = env.run_cli(flags, socket=env.default_socket)
        assert res.returncode == 1, f"{flags} expected rc=1, got {res.returncode}"
        assert CONFLICT_ERR in res.stderr, f"{flags} stderr={res.stderr!r}"


def test_t2_cli_03_unknown_cli_flag(env: SyeTestEnv) -> None:
    """Unknown CLI flag exits non-zero with error on stderr."""
    res = env.run_cli(["--definitely-invalid-flag"])
    assert res.returncode != 0, "Expected non-zero exit for unknown flag"
    assert len(res.stderr.strip()) > 0, "Expected error message on stderr"


def test_t2_cli_04_invalid_disable_argument(env: SyeTestEnv) -> None:
    """Non-numeric and negative --disable arguments are rejected."""
    for bad_arg in ("notanumber", "-5"):
        res = env.run_cli(["--disable", bad_arg], socket=env.default_socket)
        assert res.returncode != 0, f"--disable {bad_arg} should exit non-zero"
        assert len(res.stderr.strip()) > 0, f"--disable {bad_arg} should write stderr"


def test_t2_cli_05_validate_config_missing_file(env: SyeTestEnv) -> None:
    """--validate-config on a non-existent path exits 1 with stderr error."""
    missing = env.config_path("does_not_exist.toml")
    res = env.run_cli(["--validate-config"], config=missing)
    assert res.returncode == 1, f"Expected rc=1 for missing file, got {res.returncode}"
    assert len(res.stderr.strip()) > 0, "Expected error message on stderr"


def test_t2_cli_06_validate_config_malformed_toml(env: SyeTestEnv) -> None:
    """--validate-config on syntactically invalid TOML exits 1 with stderr error."""
    bad = env.write_config("short_break_interval_secs = [broken\n", name="bad.toml")
    res = env.run_cli(["--validate-config"], config=bad)
    assert res.returncode == 1, f"Expected rc=1 for malformed TOML, got {res.returncode}"
    assert len(res.stderr.strip()) > 0, "Expected error message on stderr"


# ---------------------------------------------------------------------------
# Area 2B: Config Validation & Fallback Corner Cases (6 tests)
# ---------------------------------------------------------------------------


def test_t2_cfg_01_zero_interval_rejected(env: SyeTestEnv) -> None:
    """Zero short or long break intervals fail --validate-config."""
    for field in ("short_break_interval_secs", "long_break_interval_secs"):
        cfg = env.write_profile_config({field: 0}, name=f"zero_{field}.toml")
        res = env.run_cli(["--validate-config"], config=cfg)
        assert res.returncode == 1, f"{field}=0 should exit 1, got {res.returncode}"
        assert len(res.stderr.strip()) > 0, f"{field}=0 should write stderr"


def test_t2_cfg_02_zero_duration_rejected(env: SyeTestEnv) -> None:
    """Zero short or long break durations fail --validate-config."""
    for field in ("short_break_duration_secs", "long_break_duration_secs"):
        cfg = env.write_profile_config({field: 0}, name=f"zero_{field}.toml")
        res = env.run_cli(["--validate-config"], config=cfg)
        assert res.returncode == 1, f"{field}=0 should exit 1, got {res.returncode}"
        assert len(res.stderr.strip()) > 0, f"{field}=0 should write stderr"


def test_t2_cfg_03_negative_intervals_rejected(env: SyeTestEnv) -> None:
    """Negative short or long break intervals fail --validate-config."""
    for field, bad_val in (
        ("short_break_interval_secs", -10),
        ("long_break_interval_secs", -5),
    ):
        neg_cfg = env.write_profile_config({field: bad_val}, name=f"neg_{field}.toml")
        res_neg = env.run_cli(["--validate-config"], config=neg_cfg)
        assert res_neg.returncode == 1, f"{field}={bad_val} rc={res_neg.returncode}"
        assert len(res_neg.stderr.strip()) > 0, f"{field}={bad_val} should write stderr"


def test_t2_cfg_04_pre_break_warning_ge_short_interval_rejected(env: SyeTestEnv) -> None:
    """pre_break_warning_secs >= short_break_interval_secs fails --validate-config."""
    cfg = env.write_profile_config(
        {"short_break_interval_secs": 5, "pre_break_warning_secs": 10},
        name="bad_prebreak.toml",
    )
    res = env.run_cli(["--validate-config"], config=cfg)
    assert res.returncode == 1, f"pre_break >= short_interval should exit 1, got {res.returncode}"
    assert len(res.stderr.strip()) > 0


def test_t2_cfg_05_corrupted_toml_fallback_on_daemon_startup(env: SyeTestEnv) -> None:
    """Daemon falls back to defaults on corrupted TOML instead of panicking (F10)."""
    bad = env.write_config("short_break_interval_secs = [[[corrupt\n", name="corrupt.toml")
    daemon = env.start_daemon(config=bad)
    assert daemon.poll() is None, "Daemon exited on corrupted config"
    assert "panicked at" not in daemon.read_stderr()
    st = env.status_json()
    env.assert_status_schema(st, expect_enabled=True)


def test_t2_cfg_06_missing_config_fallback_on_daemon_startup(env: SyeTestEnv) -> None:
    """Daemon starts cleanly with defaults when --config points to a non-existent file."""
    missing = env.config_path("missing_startup.toml")
    daemon = env.start_daemon(config=missing)
    assert daemon.poll() is None
    st = env.status_json()
    env.assert_status_schema(st, expect_enabled=True)


# ---------------------------------------------------------------------------
# Area 2C: Daemon-Not-Running & Stale Socket Recovery (6 tests)
# ---------------------------------------------------------------------------


def test_t2_ipc_01_status_when_not_running(env: SyeTestEnv) -> None:
    """--status and --status --json exit 1 with contract message when daemon is absent."""
    unused = env.alloc_socket()
    for args in (["--status"], ["--status", "--json"]):
        res = env.run_cli(args, socket=unused)
        assert res.returncode == 1, f"{args} expected rc=1, got {res.returncode}"
        assert NOT_RUNNING_ERR in res.stderr, f"{args} stderr={res.stderr!r}"


def test_t2_ipc_02_enable_disable_when_not_running(env: SyeTestEnv) -> None:
    """-e, -d, and -d 5 exit 1 with a non-empty stderr message when daemon is absent."""
    unused = env.alloc_socket()
    for args in (["-e"], ["-d"], ["-d", "5"]):
        res = env.run_cli(args, socket=unused)
        assert res.returncode == 1, f"{args} expected rc=1, got {res.returncode}"
        assert len(res.stderr.strip()) > 0, f"{args} expected non-empty stderr, got {res.stderr!r}"


def test_t2_ipc_03_manual_break_flags_when_not_running(env: SyeTestEnv) -> None:
    """-t, -b, and -l exit 1 with a non-empty stderr message when daemon is absent."""
    unused = env.alloc_socket()
    for args in (["-t"], ["-b"], ["-l"]):
        res = env.run_cli(args, socket=unused)
        assert res.returncode == 1, f"{args} expected rc=1, got {res.returncode}"
        assert len(res.stderr.strip()) > 0, f"{args} expected non-empty stderr, got {res.stderr!r}"


def test_t2_ipc_04_quit_when_not_running(env: SyeTestEnv) -> None:
    """-q and --quit exit 1 with a non-empty stderr message when daemon is absent."""
    unused = env.alloc_socket()
    for args in (["-q"], ["--quit"]):
        res = env.run_cli(args, socket=unused)
        assert res.returncode == 1, f"{args} expected rc=1, got {res.returncode}"
        assert len(res.stderr.strip()) > 0, f"{args} expected non-empty stderr, got {res.stderr!r}"


def test_t2_ipc_05_stale_socket_recovery_after_sigkill(env: SyeTestEnv) -> None:
    """Daemon recovers stale UDS file left behind after SIGKILL."""
    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    d1 = env.start_daemon(config=cfg)
    d1.proc.kill()
    d1.proc.wait(timeout=5.0)

    d2 = env.start_daemon(config=cfg, socket=env.default_socket)
    assert d2.poll() is None
    st = env.status_json()
    env.assert_status_schema(st, expect_enabled=True)


def test_t2_ipc_06_preexisting_dead_uds_file_recovery(env: SyeTestEnv) -> None:
    """Daemon recovers a pre-existing bound-then-closed dead AF_UNIX socket file."""
    if os.name == "nt":
        return
    sock_path = env.alloc_socket()
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        s.bind(str(sock_path))
    finally:
        s.close()
    assert sock_path.exists(), "Pre-existing dead socket file was not created"

    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    daemon = env.start_daemon(config=cfg, socket=sock_path)
    assert daemon.poll() is None
    st = env.status_json(socket=sock_path)
    env.assert_status_schema(st, expect_enabled=True)


# ---------------------------------------------------------------------------
# Area 2D: Scheduler & Empty/Extreme Break Config Edge Cases (6 tests)
# ---------------------------------------------------------------------------


def test_t2_sch_01_empty_breaks_lists_no_panic(env: SyeTestEnv) -> None:
    """Empty short_breaks and long_breaks never cause a panic or crash signal."""
    cfg = env.write_profile_config(
        {
            "short_breaks": [],
            "long_breaks": [],
            "short_break_interval_secs": 5,
            "pre_break_warning_secs": 0,
        }
    )
    daemon = env.start_daemon(config=cfg, wait_ready=False)
    time.sleep(0.5)
    rc = daemon.poll()
    stderr = daemon.read_stderr()
    assert "panicked at" not in stderr, f"Daemon panicked on empty breaks: {stderr!r}"
    if rc is None:
        st = env.status_json()
        norm = env.assert_status_schema(st)
        assert st["next_break_type"] is None or norm == "stopped", (
            f"Unexpected active schedule with empty breaks: {st}"
        )
    else:
        assert rc >= 0, f"Daemon crashed with signal {-rc}: {stderr!r}"


def test_t2_sch_02_short_breaks_only(env: SyeTestEnv) -> None:
    """Daemon schedules and completes short breaks cleanly when long_breaks is empty."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 2,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 10,
            "long_break_duration_secs": 1,
            "pre_break_warning_secs": 0,
            "random_order": False,
            "short_breaks": [{"name": "OnlyShort1"}, {"name": "OnlyShort2"}],
            "long_breaks": [],
        }
    )
    daemon = env.start_daemon(config=cfg)
    st = env.status_json()
    assert st["next_break_type"] == "short"
    env.wait_for_state("break", timeout=5.0, interval=0.08)
    st_after = env.wait_for_state("waiting", timeout=5.0, interval=0.08)
    assert st_after["next_break_type"] == "short"
    assert "panicked at" not in daemon.read_stderr()


def test_t2_sch_03_long_breaks_only(env: SyeTestEnv) -> None:
    """Daemon schedules and completes long breaks cleanly when short_breaks is empty."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 2,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 4,
            "long_break_duration_secs": 1,
            "pre_break_warning_secs": 0,
            "random_order": False,
            "short_breaks": [],
            "long_breaks": [{"name": "OnlyLong1"}, {"name": "OnlyLong2"}],
        }
    )
    daemon = env.start_daemon(config=cfg)
    st = env.status_json()
    assert st["next_break_type"] == "long"
    env.wait_for_state("break", timeout=7.0, interval=0.08)
    st_after = env.wait_for_state("waiting", timeout=5.0, interval=0.08)
    assert st_after["next_break_type"] == "long"
    assert "panicked at" not in daemon.read_stderr()


def test_t2_sch_04_minimum_valid_intervals_rapid_cycles(env: SyeTestEnv) -> None:
    """Rapid 2s/1s breaks with pre_break_warning_secs=0 complete multiple cycles cleanly."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 2,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 60,
            "long_break_duration_secs": 1,
            "pre_break_warning_secs": 0,
            "random_order": False,
            "short_breaks": [{"name": "Fast1"}, {"name": "Fast2"}],
        }
    )
    daemon = env.start_daemon(config=cfg)
    for _ in range(2):
        env.wait_for_state("break", timeout=5.0, interval=0.08)
        env.wait_for_state("waiting", timeout=5.0, interval=0.08)
    assert daemon.poll() is None


def test_t2_sch_05_extreme_large_intervals(env: SyeTestEnv) -> None:
    """Large 1-day/1-week intervals validate and run cleanly without overflow."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 86400,
            "short_break_duration_secs": 15,
            "long_break_interval_secs": 604800,
            "long_break_duration_secs": 60,
            "pre_break_warning_secs": 10,
        }
    )
    assert env.run_cli(["--validate-config"], config=cfg).returncode == 0
    env.start_daemon(config=cfg)
    st = env.status_json()
    env.assert_status_schema(st, expect_enabled=True)
    secs = st["seconds_until_next_break"]
    assert isinstance(secs, int) and 85000 <= secs <= 86400, f"Unexpected countdown: {secs}"


def test_t2_sch_06_per_break_override_config(env: SyeTestEnv) -> None:
    """Per-break override fields on BreakSpec validate and run cleanly."""
    base = env.default_config_dict()
    sample_break = base["short_breaks"][0] if base.get("short_breaks") else {"name": "OverrideEx"}
    custom_break = dict(sample_break)
    name_key = "title" if "title" in sample_break and "name" not in sample_break else "name"
    custom_break[name_key] = "Override Short"
    override_key = (
        "duration"
        if "duration" in sample_break and "duration_secs" not in sample_break
        else "duration_secs"
    )
    custom_break[override_key] = 2

    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 30,
            "short_break_duration_secs": 5,
            "short_breaks": [custom_break],
        }
    )
    val = env.run_cli(["--validate-config"], config=cfg)
    assert val.returncode == 0, f"Per-break override rejected: stderr={val.stderr!r}"
    env.start_daemon(config=cfg)
    st = env.status_json()
    assert st["next_break_name"] == "Override Short"


# ---------------------------------------------------------------------------
# Area 2E: State Transitions & Rapid IPC Concurrency Edge Cases (6 tests)
# ---------------------------------------------------------------------------


def test_t2_con_01_rapid_sequential_ipc_toggle_storm(env: SyeTestEnv) -> None:
    """10 rapid sequential --disable/--enable toggles leave daemon healthy & enabled."""
    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    env.start_daemon(config=cfg)
    for _ in range(5):
        assert env.run_cli(["--disable"], socket=env.default_socket).returncode == 0
        assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    norm = env.assert_status_schema(st, expect_enabled=True)
    assert norm == "waiting"


def test_t2_con_02_idempotent_enable_and_disable(env: SyeTestEnv) -> None:
    """Repeated --enable and repeated --disable are idempotent and exit 0."""
    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    env.start_daemon(config=cfg)

    for _ in range(2):
        assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
        assert env.status_json()["enabled"] is True

    for _ in range(2):
        assert env.run_cli(["--disable"], socket=env.default_socket).returncode == 0
        assert env.status_json()["enabled"] is False


def test_t2_con_03_manual_break_while_disabled(env: SyeTestEnv) -> None:
    """Sending --take-break or -b while Stopped does not crash or hang the daemon."""
    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    daemon = env.start_daemon(config=cfg)
    assert env.run_cli(["--disable"], socket=env.default_socket).returncode == 0

    env.run_cli(["--take-break"], socket=env.default_socket)
    env.run_cli(["-b"], socket=env.default_socket)
    assert daemon.poll() is None
    assert "panicked at" not in daemon.read_stderr()

    assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    assert st["enabled"] is True


def test_t2_con_04_manual_break_while_already_in_break(env: SyeTestEnv) -> None:
    """Sending -t while already in Break causes no crash and returns to Waiting."""
    cfg = env.write_profile_config(
        {"short_break_interval_secs": 30, "short_break_duration_secs": 2}
    )
    daemon = env.start_daemon(config=cfg)
    assert env.run_cli(["-b"], socket=env.default_socket).returncode == 0
    env.wait_for_state("break", timeout=3.0, interval=0.08)

    env.run_cli(["-t"], socket=env.default_socket)
    assert daemon.poll() is None
    env.wait_for_state("waiting", timeout=6.0, interval=0.1)
    assert "panicked at" not in daemon.read_stderr()


def test_t2_con_05_concurrent_status_json_clients(env: SyeTestEnv) -> None:
    """5 simultaneous --status --json subprocesses all succeed with valid JSON."""
    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    env.start_daemon(config=cfg)

    def _query() -> tuple[int, dict]:
        res = env.run_cli(["--status", "--json"], socket=env.default_socket)
        return res.returncode, json.loads(res.stdout) if res.returncode == 0 else {}

    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:
        results = list(pool.map(lambda _: _query(), range(5)))

    for rc, data in results:
        assert rc == 0, f"Concurrent --status --json failed: rc={rc}"
        env.assert_status_schema(data, expect_enabled=True)


def test_t2_con_06_override_timed_disable_with_indefinite_disable(env: SyeTestEnv) -> None:
    """--disable 10 followed by --disable 0 stays disabled until --enable."""
    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    env.start_daemon(config=cfg)

    assert env.run_cli(["--disable", "10"], socket=env.default_socket).returncode == 0
    assert env.status_json()["enabled"] is False

    assert env.run_cli(["--disable", "0"], socket=env.default_socket).returncode == 0
    assert env.status_json()["enabled"] is False

    assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    assert st["enabled"] is True and normalize_state(st["state"]) == "waiting"


TESTS = [
    test_t2_cli_01_conflicting_short_and_long_break_flags,
    test_t2_cli_02_conflicting_long_and_mixed_break_flags,
    test_t2_cli_03_unknown_cli_flag,
    test_t2_cli_04_invalid_disable_argument,
    test_t2_cli_05_validate_config_missing_file,
    test_t2_cli_06_validate_config_malformed_toml,
    test_t2_cfg_01_zero_interval_rejected,
    test_t2_cfg_02_zero_duration_rejected,
    test_t2_cfg_03_negative_intervals_rejected,
    test_t2_cfg_04_pre_break_warning_ge_short_interval_rejected,
    test_t2_cfg_05_corrupted_toml_fallback_on_daemon_startup,
    test_t2_cfg_06_missing_config_fallback_on_daemon_startup,
    test_t2_ipc_01_status_when_not_running,
    test_t2_ipc_02_enable_disable_when_not_running,
    test_t2_ipc_03_manual_break_flags_when_not_running,
    test_t2_ipc_04_quit_when_not_running,
    test_t2_ipc_05_stale_socket_recovery_after_sigkill,
    test_t2_ipc_06_preexisting_dead_uds_file_recovery,
    test_t2_sch_01_empty_breaks_lists_no_panic,
    test_t2_sch_02_short_breaks_only,
    test_t2_sch_03_long_breaks_only,
    test_t2_sch_04_minimum_valid_intervals_rapid_cycles,
    test_t2_sch_05_extreme_large_intervals,
    test_t2_sch_06_per_break_override_config,
    test_t2_con_01_rapid_sequential_ipc_toggle_storm,
    test_t2_con_02_idempotent_enable_and_disable,
    test_t2_con_03_manual_break_while_disabled,
    test_t2_con_04_manual_break_while_already_in_break,
    test_t2_con_05_concurrent_status_json_clients,
    test_t2_con_06_override_timed_disable_with_indefinite_disable,
]
