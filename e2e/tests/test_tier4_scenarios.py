"""Tier 4: Real-world end-user workload scenario E2E tests (6 scenarios)."""

from __future__ import annotations

import json
import os
import socket
import threading
import time

from e2e.harness import SyeTestEnv, normalize_state, parse_toml


def test_t4_01_full_workday_short_short_long_cycle(env: SyeTestEnv) -> None:
    """Scenario 1: Developer runs a continuous workday through short->short->long->short."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 3,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 9,
            "long_break_duration_secs": 1,
            "pre_break_warning_secs": 1,
            "random_order": False,
            "persist_state": False,
            "short_breaks": [
                {"name": "Workday Short 1"},
                {"name": "Workday Short 2"},
                {"name": "Workday Short 3"},
            ],
            "long_breaks": [
                {"name": "Workday Long 1"},
                {"name": "Workday Long 2"},
            ],
        }
    )
    daemon = env.start_daemon(config=cfg)

    expected = [
        ("short", "Workday Short 1"),
        ("short", "Workday Short 2"),
        ("long", "Workday Long 1"),
        ("short", "Workday Short 3"),
    ]
    seen_states: set[str] = set()
    seen_breaks: list[tuple[str, str]] = []

    deadline = time.monotonic() + 26.0
    while time.monotonic() < deadline:
        st = env.status_json()
        norm = normalize_state(st["state"])
        seen_states.add(norm)
        if norm in ("waiting", "prebreak"):
            pair = (st["next_break_type"], st["next_break_name"])
            if pair[0] and pair[1] and (not seen_breaks or seen_breaks[-1] != pair):
                seen_breaks.append(pair)
        if len(seen_breaks) >= len(expected):
            break
        time.sleep(0.1)

    assert {"waiting", "prebreak", "break"}.issubset(seen_states), (
        f"Expected waiting, prebreak, break in workday cycle; got {seen_states}"
    )
    assert seen_breaks[: len(expected)] == expected, (
        f"Expected workday progression {expected}, got {seen_breaks}"
    )
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0
    assert daemon.proc.wait(timeout=5.0) == 0


def test_t4_02_meeting_pause_and_resume_workflow(env: SyeTestEnv) -> None:
    """Scenario 2: User pauses for a 30m video call, checks status, resumes, takes break."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 30,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 120,
            "long_break_duration_secs": 2,
            "pre_break_warning_secs": 2,
            "random_order": False,
        }
    )
    daemon = env.start_daemon(config=cfg)
    assert env.assert_status_schema(env.status_json(), expect_enabled=True) == "waiting"

    assert env.run_cli(["--disable", "30"], socket=env.default_socket).returncode == 0
    assert env.assert_status_schema(env.status_json(), expect_enabled=False) == "stopped"

    human = env.run_cli(["--status"], socket=env.default_socket)
    assert human.returncode == 0 and len(human.stdout.strip()) > 0

    time.sleep(1.2)
    st_mid = env.status_json()
    assert st_mid["enabled"] is False and normalize_state(st_mid["state"]) == "stopped"

    assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
    assert env.assert_status_schema(env.status_json(), expect_enabled=True) == "waiting"

    assert env.run_cli(["-b"], socket=env.default_socket).returncode == 0
    env.wait_for_state("break", timeout=3.0, interval=0.08)
    env.wait_for_state("waiting", timeout=5.0, interval=0.08)

    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0
    assert daemon.proc.wait(timeout=5.0) == 0


def test_t4_03_adhoc_manual_breaks_conflict_guard_and_clean_quit(
    env: SyeTestEnv,
) -> None:
    """Scenario 3: User runs -b, -l, -t, hits -b -l guard, and quits cleanly."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 30,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 90,
            "long_break_duration_secs": 1,
            "pre_break_warning_secs": 2,
            "random_order": False,
            "short_breaks": [{"name": "Adhoc Short 1"}, {"name": "Adhoc Short 2"}],
            "long_breaks": [{"name": "Adhoc Long 1"}],
        }
    )
    daemon = env.start_daemon(config=cfg)
    assert normalize_state(env.status_json()["state"]) == "waiting"

    for flag in ("-b", "-l", "-t"):
        res = env.run_cli([flag], socket=env.default_socket)
        assert res.returncode == 0, f"{flag} failed: stderr={res.stderr!r}"
        env.wait_for_state("break", timeout=2.5, interval=0.08)
        env.wait_for_state("waiting", timeout=5.0, interval=0.08)

    conflict = env.run_cli(["-b", "-l"], socket=env.default_socket)
    assert conflict.returncode == 1
    assert "Cannot combine -b/--short-break with -l/--long-break" in conflict.stderr
    assert env.status_json()["enabled"] is True

    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0
    assert daemon.proc.wait(timeout=5.0) == 0

    after = env.run_cli(["--status"], socket=env.default_socket)
    assert after.returncode == 1
    assert "Safe Your Eyes is not running" in after.stderr


def test_t4_04_config_customization_validation_bad_edit_and_recovery(
    env: SyeTestEnv,
) -> None:
    """Scenario 4: Power user exports defaults, customizes, catches bad edits, recovers."""
    printed = env.run_cli(["--print-default-config"])
    assert printed.returncode == 0
    default_cfg = parse_toml(printed.stdout)
    assert "short_break_interval_secs" in default_cfg

    cfg_path = env.write_profile_config(
        {
            "short_break_interval_secs": 25,
            "short_break_duration_secs": 1,
            "random_order": False,
            "short_breaks": [{"name": "Custom Stretch A"}],
            "long_breaks": [{"name": "Custom Long A"}],
        }
    )
    assert env.run_cli(["--validate-config"], config=cfg_path).returncode == 0

    d1 = env.start_daemon(config=cfg_path)
    st1 = env.status_json()
    assert st1["next_break_name"] == "Custom Stretch A"
    assert 0 <= st1["seconds_until_next_break"] <= 25
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0
    assert d1.proc.wait(timeout=5.0) == 0

    bad_syntax = env.write_config(
        'short_break_interval_secs = "not_a_number"\n', name="bad_syntax.toml"
    )
    res_syn = env.run_cli(["--validate-config"], config=bad_syntax)
    assert res_syn.returncode == 1 and len(res_syn.stderr.strip()) > 0

    bad_val = env.write_profile_config(
        {"short_break_interval_secs": 0}, name="bad_val.toml"
    )
    res_val = env.run_cli(["--validate-config"], config=bad_val)
    assert res_val.returncode == 1 and len(res_val.stderr.strip()) > 0

    recovered_path = env.write_profile_config(
        {
            "short_break_interval_secs": 25,
            "short_break_duration_secs": 1,
            "random_order": False,
            "short_breaks": [{"name": "Recovered Exercise B"}],
            "long_breaks": [{"name": "Custom Long A"}],
        }
    )
    assert env.run_cli(["--validate-config"], config=recovered_path).returncode == 0

    d2 = env.start_daemon(config=recovered_path)
    assert env.status_json()["next_break_name"] == "Recovered Exercise B"
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0
    assert d2.proc.wait(timeout=5.0) == 0


def test_t4_05_crash_sigkill_and_stale_socket_recovery(env: SyeTestEnv) -> None:
    """Scenario 5: Daemon receives SIGKILL, leaves stale socket, relaunch recovers cleanly."""
    cfg = env.write_profile_config({"short_break_interval_secs": 30})
    sock = env.default_socket
    d1 = env.start_daemon(config=cfg, socket=sock)
    assert env.status_json(socket=sock)["enabled"] is True

    d1.proc.kill()
    d1.proc.wait(timeout=5.0)

    if os.name != "nt" and not sock.exists():
        # Ensure a stale dead socket file is present to definitely exercise recovery.
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            s.bind(str(sock))
        finally:
            s.close()

    dead_status = env.run_cli(["--status"], socket=sock)
    assert dead_status.returncode == 1
    assert "Safe Your Eyes is not running" in dead_status.stderr

    d2 = env.start_daemon(config=cfg, socket=sock)
    assert d2.poll() is None
    st2 = env.status_json(socket=sock)
    assert env.assert_status_schema(st2, expect_enabled=True) == "waiting"

    assert env.run_cli(["--quit"], socket=sock).returncode == 0
    assert d2.proc.wait(timeout=5.0) == 0


def test_t4_06_multi_client_status_bar_polling_and_control_commands(
    env: SyeTestEnv,
) -> None:
    """Scenario 6: Continuous status-bar pollers run concurrently with CLI keybindings."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 30,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 120,
            "long_break_duration_secs": 2,
            "pre_break_warning_secs": 2,
        }
    )
    daemon = env.start_daemon(config=cfg)

    stop_evt = threading.Event()
    errors: list[str] = []
    observed_states: set[str] = set()
    lock = threading.Lock()

    def _poller() -> None:
        while not stop_evt.is_set():
            res = env.run_cli(["--status", "--json"], socket=env.default_socket)
            if res.returncode != 0:
                with lock:
                    errors.append(f"rc={res.returncode} stderr={res.stderr!r}")
            else:
                try:
                    data = json.loads(res.stdout)
                    with lock:
                        observed_states.add(normalize_state(data["state"]))
                except Exception as exc:  # noqa: BLE001
                    with lock:
                        errors.append(f"parse error: {exc!r} stdout={res.stdout!r}")
            time.sleep(0.2)

    threads = [threading.Thread(target=_poller, daemon=True) for _ in range(2)]
    for t in threads:
        t.start()

    try:
        time.sleep(0.4)
        assert env.run_cli(["--disable", "10"], socket=env.default_socket).returncode == 0
        time.sleep(0.5)
        assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
        time.sleep(0.4)
        assert env.run_cli(["-b"], socket=env.default_socket).returncode == 0
        env.wait_for_state("break", timeout=2.5, interval=0.08)
        time.sleep(0.4)
        env.wait_for_state("waiting", timeout=5.0, interval=0.08)
        dup = env.run_cli(["--headless"], config=cfg, socket=env.default_socket)
        assert dup.returncode == 0
        assert "Safe Your Eyes is already running" in dup.stdout
    finally:
        stop_evt.set()
        for t in threads:
            t.join(timeout=5.0)

    assert not errors, f"Status bar pollers observed failures: {errors[:5]}"
    assert {"waiting", "stopped", "break"}.issubset(observed_states), (
        f"Expected waiting, stopped, break in poller trace; got {observed_states}"
    )
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0
    assert daemon.proc.wait(timeout=5.0) == 0


TESTS = [
    test_t4_01_full_workday_short_short_long_cycle,
    test_t4_02_meeting_pause_and_resume_workflow,
    test_t4_03_adhoc_manual_breaks_conflict_guard_and_clean_quit,
    test_t4_04_config_customization_validation_bad_edit_and_recovery,
    test_t4_05_crash_sigkill_and_stale_socket_recovery,
    test_t4_06_multi_client_status_bar_polling_and_control_commands,
]
