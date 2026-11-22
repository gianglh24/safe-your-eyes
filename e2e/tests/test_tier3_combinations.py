"""Tier 3: Pairwise cross-feature combination opaque-box E2E tests (8 tests)."""

from __future__ import annotations

import concurrent.futures
import json
import time

from e2e.harness import SyeTestEnv, normalize_state


def _fast_manual_overrides() -> dict:
    return {
        "short_break_interval_secs": 30,
        "short_break_duration_secs": 1,
        "long_break_interval_secs": 90,
        "long_break_duration_secs": 2,
        "pre_break_warning_secs": 2,
        "strict_break": False,
        "allow_postpone": False,
        "random_order": False,
        "persist_state": False,
        "short_breaks": [
            {"name": "Manual Short 1"},
            {"name": "Manual Short 2"},
            {"name": "Manual Short 3"},
        ],
        "long_breaks": [
            {"name": "Manual Long 1"},
            {"name": "Manual Long 2"},
        ],
    }


def test_t3_01_config_overrides_status_timed_and_indefinite_disable_enable(
    env: SyeTestEnv,
) -> None:
    """Config overrides + --status --json + --disable <MIN> + --disable + --enable."""
    cfg = env.write_profile_config(_fast_manual_overrides())
    env.start_daemon(config=cfg)

    st = env.status_json()
    assert env.assert_status_schema(st, expect_enabled=True) == "waiting"
    assert st["next_break_type"] == "short"
    assert st["next_break_name"] == "Manual Short 1"
    assert 0 <= st["seconds_until_next_break"] <= 30

    assert env.run_cli(["--disable", "5"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    assert env.assert_status_schema(st, expect_enabled=False) == "stopped"

    assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    assert env.assert_status_schema(st, expect_enabled=True) == "waiting"

    assert env.run_cli(["--disable"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    assert env.assert_status_schema(st, expect_enabled=False) == "stopped"

    assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    assert env.assert_status_schema(st, expect_enabled=True) == "waiting"

    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0


def test_t3_02_strict_break_with_manual_take_break(env: SyeTestEnv) -> None:
    """strict_break=true + manual -t break advances queue with consecutive_skips=0."""
    overrides = _fast_manual_overrides()
    overrides["strict_break"] = True
    cfg = env.write_profile_config(overrides)
    assert env.run_cli(["--validate-config"], config=cfg).returncode == 0

    env.start_daemon(config=cfg)
    st0 = env.status_json()
    assert env.assert_status_schema(st0, expect_enabled=True) == "waiting"
    assert st0["consecutive_skips"] == 0

    assert env.run_cli(["-t"], socket=env.default_socket).returncode == 0
    env.wait_for_state("break", timeout=2.5, interval=0.08)
    st1 = env.wait_for_state("waiting", timeout=5.0, interval=0.08)
    assert st1["next_break_name"] == "Manual Short 2", f"Queue did not advance: {st1}"
    assert st1["consecutive_skips"] == 0
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0


def test_t3_03_short_long_cycle_and_state_transitions(env: SyeTestEnv) -> None:
    """Natural timer drives Waiting->PreBreak->Break->Waiting and short->short->long->short."""
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
                {"name": "Short Exercise 1"},
                {"name": "Short Exercise 2"},
                {"name": "Short Exercise 3"},
            ],
            "long_breaks": [
                {"name": "Long Exercise 1"},
                {"name": "Long Exercise 2"},
            ],
        }
    )
    env.start_daemon(config=cfg)

    expected_breaks = [
        ("short", "Short Exercise 1"),
        ("short", "Short Exercise 2"),
        ("long", "Long Exercise 1"),
        ("short", "Short Exercise 3"),
    ]
    states_seen: list[str] = []
    breaks_seen: list[tuple[str, str]] = []

    deadline = time.monotonic() + 26.0
    while time.monotonic() < deadline:
        st = env.status_json()
        norm = normalize_state(st["state"])
        if not states_seen or states_seen[-1] != norm:
            states_seen.append(norm)
        if norm in ("waiting", "prebreak"):
            pair = (st["next_break_type"], st["next_break_name"])
            if pair[0] and pair[1] and (not breaks_seen or breaks_seen[-1] != pair):
                breaks_seen.append(pair)
        if len(breaks_seen) >= len(expected_breaks):
            break
        time.sleep(0.1)

    # Verify Waiting -> PreBreak -> Break -> Waiting subsequence appeared
    subseq = ["waiting", "prebreak", "break", "waiting"]
    idx = 0
    for s in states_seen:
        if s == subseq[idx]:
            idx += 1
            if idx == len(subseq):
                break
    assert idx == len(subseq), f"Missing state transition cycle {subseq} in {states_seen}"
    assert breaks_seen[: len(expected_breaks)] == expected_breaks, (
        f"Expected break sequence {expected_breaks}, observed {breaks_seen}"
    )
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0


def test_t3_04_concurrent_clients_and_duplicate_launch_guard(env: SyeTestEnv) -> None:
    """12 concurrent --status --json clients + 2 duplicate launches succeed cleanly."""
    cfg = env.write_profile_config(_fast_manual_overrides())
    env.start_daemon(config=cfg)

    def _status_task(_: int) -> tuple[str, int, str]:
        res = env.run_cli(["--status", "--json"], socket=env.default_socket)
        return ("status", res.returncode, res.stdout)

    def _dup_task(_: int) -> tuple[str, int, str]:
        res = env.run_cli(["--headless"], config=cfg, socket=env.default_socket)
        return ("dup", res.returncode, res.stdout)

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        futs = [pool.submit(_status_task, i) for i in range(12)]
        futs += [pool.submit(_dup_task, i) for i in range(2)]
        results = [f.result(timeout=15.0) for f in futs]

    for kind, rc, out in results:
        assert rc == 0, f"{kind} task failed with rc={rc}, stdout={out!r}"
        if kind == "status":
            env.assert_status_schema(json.loads(out), expect_enabled=True)
        else:
            assert "Safe Your Eyes is already running" in out, (
                f"Duplicate launch missing expected message: {out!r}"
            )

    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0


def test_t3_05_persist_state_true_vs_false_across_restart(env: SyeTestEnv) -> None:
    """persist_state=true preserves queue position across restart; false resets it."""
    # Part A: persist_state = true
    cfg_persist = env.write_profile_config(
        {
            "short_break_interval_secs": 30,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 300,
            "long_break_duration_secs": 1,
            "pre_break_warning_secs": 0,
            "random_order": False,
            "persist_state": True,
            "short_breaks": [
                {"name": "Persist Short 1"},
                {"name": "Persist Short 2"},
                {"name": "Persist Short 3"},
            ],
            "long_breaks": [{"name": "Persist Long 1"}],
        },
        name="persist.toml",
    )
    sock_a = env.alloc_socket()
    d1 = env.start_daemon(config=cfg_persist, socket=sock_a)
    assert env.status_json(socket=sock_a)["next_break_name"] == "Persist Short 1"
    assert env.run_cli(["-b"], socket=sock_a).returncode == 0
    env.wait_for_state("break", socket=sock_a, timeout=3.0, interval=0.08)
    st_after = env.wait_for_state("waiting", socket=sock_a, timeout=5.0, interval=0.08)
    assert st_after["next_break_name"] == "Persist Short 2"
    assert env.run_cli(["--quit"], socket=sock_a).returncode == 0
    assert d1.proc.wait(timeout=5.0) == 0

    d2 = env.start_daemon(config=cfg_persist, socket=sock_a)
    st_restarted = env.status_json(socket=sock_a)
    assert st_restarted["next_break_name"] == "Persist Short 2", (
        f"Expected persisted queue position 'Persist Short 2', got {st_restarted}"
    )
    assert env.run_cli(["--quit"], socket=sock_a).returncode == 0
    assert d2.proc.wait(timeout=5.0) == 0

    # Part B: persist_state = false
    cfg_nopersist = env.write_profile_config(
        _fast_manual_overrides(), name="nopersist.toml"
    )
    sock_b = env.alloc_socket()
    d3 = env.start_daemon(config=cfg_nopersist, socket=sock_b)
    assert env.status_json(socket=sock_b)["next_break_name"] == "Manual Short 1"
    assert env.run_cli(["-b"], socket=sock_b).returncode == 0
    env.wait_for_state("break", socket=sock_b, timeout=3.0, interval=0.08)
    env.wait_for_state("waiting", socket=sock_b, timeout=5.0, interval=0.08)
    assert env.run_cli(["--quit"], socket=sock_b).returncode == 0
    assert d3.proc.wait(timeout=5.0) == 0

    d4 = env.start_daemon(config=cfg_nopersist, socket=sock_b)
    st_reset = env.status_json(socket=sock_b)
    assert st_reset["next_break_name"] == "Manual Short 1", (
        f"Expected reset to 'Manual Short 1' with persist_state=false, got {st_reset}"
    )
    assert env.run_cli(["--quit"], socket=sock_b).returncode == 0
    assert d4.proc.wait(timeout=5.0) == 0


def test_t3_06_policy_bundle_all_enabled_headless(env: SyeTestEnv) -> None:
    """All 4 policy tables enabled simultaneously validate and run cleanly headless."""
    overrides = _fast_manual_overrides()
    overrides.update(
        {
            "smart_pause": {
                "enabled": True,
                "idle_threshold_secs": 300,
                "postpone_if_active": True,
            },
            "do_not_disturb": {
                "enabled": True,
                "skip_break_windows": ["mpv", "vlc"],
                "take_break_windows": ["code"],
                "while_on_battery": False,
            },
            "audible_alert": {
                "pre_break": True,
                "post_break": True,
                "volume": 50,
            },
            "limit_consecutive_skips": {
                "enabled": True,
                "max_skips": 2,
            },
        }
    )
    cfg = env.write_profile_config(overrides, disable_policies=False)
    val = env.run_cli(["--validate-config"], config=cfg)
    assert val.returncode == 0, f"Policy bundle config rejected: {val.stderr!r}"

    daemon = env.start_daemon(config=cfg)
    st = env.status_json()
    assert env.assert_status_schema(st, expect_enabled=True) == "waiting"
    assert st["consecutive_skips"] == 0

    assert env.run_cli(["-b"], socket=env.default_socket).returncode == 0
    env.wait_for_state("break", timeout=3.0, interval=0.08)
    env.wait_for_state("waiting", timeout=5.0, interval=0.08)
    assert daemon.poll() is None
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0


def test_t3_07_deterministic_sequential_wrap_around(env: SyeTestEnv) -> None:
    """random_order=false wraps cleanly Seq-1 -> Seq-2 -> Seq-3 -> Seq-1."""
    cfg = env.write_profile_config(
        {
            "short_break_interval_secs": 30,
            "short_break_duration_secs": 1,
            "long_break_interval_secs": 300,
            "long_break_duration_secs": 2,
            "pre_break_warning_secs": 1,
            "random_order": False,
            "short_breaks": [
                {"name": "Seq-1"},
                {"name": "Seq-2"},
                {"name": "Seq-3"},
            ],
            "long_breaks": [{"name": "Seq-Long-1"}],
        }
    )
    env.start_daemon(config=cfg)
    assert env.status_json()["next_break_name"] == "Seq-1"

    for expected in ("Seq-2", "Seq-3", "Seq-1"):
        assert env.run_cli(["-b"], socket=env.default_socket).returncode == 0
        env.wait_for_state("break", timeout=3.0, interval=0.08)
        st = env.wait_for_state("waiting", timeout=5.0, interval=0.08)
        assert st["next_break_name"] == expected, (
            f"Expected {expected!r}, got {st['next_break_name']!r}"
        )
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0


def test_t3_08_disabled_daemon_manual_break_and_reenable_recovery(
    env: SyeTestEnv,
) -> None:
    """Disabled daemon handles -t without crashing and recovers cleanly to waiting after --enable."""
    cfg = env.write_profile_config(_fast_manual_overrides())
    daemon = env.start_daemon(config=cfg)

    assert env.run_cli(["--disable"], socket=env.default_socket).returncode == 0
    st = env.status_json()
    assert env.assert_status_schema(st, expect_enabled=False) == "stopped"

    env.run_cli(["-t"], socket=env.default_socket)
    assert daemon.poll() is None, "Daemon exited after -t while Stopped"
    assert "panicked at" not in daemon.read_stderr()
    env.assert_status_schema(env.status_json())

    assert env.run_cli(["--enable"], socket=env.default_socket).returncode == 0
    st_en = env.wait_for_state("waiting", timeout=5.0, interval=0.08)
    assert env.assert_status_schema(st_en, expect_enabled=True) == "waiting"

    assert env.run_cli(["-t"], socket=env.default_socket).returncode == 0
    env.wait_for_state("break", timeout=3.0, interval=0.08)
    env.wait_for_state("waiting", timeout=5.0, interval=0.08)
    assert env.run_cli(["--quit"], socket=env.default_socket).returncode == 0


TESTS = [
    test_t3_01_config_overrides_status_timed_and_indefinite_disable_enable,
    test_t3_02_strict_break_with_manual_take_break,
    test_t3_03_short_long_cycle_and_state_transitions,
    test_t3_04_concurrent_clients_and_duplicate_launch_guard,
    test_t3_05_persist_state_true_vs_false_across_restart,
    test_t3_06_policy_bundle_all_enabled_headless,
    test_t3_07_deterministic_sequential_wrap_around,
    test_t3_08_disabled_daemon_manual_break_and_reenable_recovery,
]
