#!/usr/bin/env python3
"""Two real native humans plus one ordinary AI; no gameplay-state injection."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SERVICE = "https://open-bevy-signaling.xiongchenyu6.workers.dev"


def fetch(url):
    request = urllib.request.Request(url, headers={
        "User-Agent": "open-bevy-read-only-qa/1", "Accept": "application/json"})
    with urllib.request.urlopen(request, timeout=15) as response:
        return json.load(response)


def read_json(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def atomic_json(path, data):
    temporary = path.with_suffix(path.suffix + ".pending")
    temporary.write_text(json.dumps(data, indent=2) + "\n")
    temporary.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/bevy-open-rts")
    parser.add_argument("--service", default=DEFAULT_SERVICE)
    parser.add_argument("--run-id", default=f"human-ai-{int(time.time())}-{os.getpid()}")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=240)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve(strict=True)
    with binary.open("rb") as stream:
        binary_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    config = fetch(args.service + "/v1/config")
    turns = [server for server in config.get("ice_servers", [])
             if any(url.startswith(("turn:", "turns:")) for url in server.get("urls", []))]
    bootstrap = {
        "health": fetch(args.service + "/healthz"),
        "session_protocol_supported": config["min_session_protocol"] <= 1 <= config["max_session_protocol"],
        "wss": config["websocket_base_url"].startswith("wss://"),
        "turn_available": bool(turns),
        "turn_credentials_present": all(bool(s.get("username")) and bool(s.get("credential")) for s in turns),
    }
    atomic_json(output / "bootstrap.json", bootstrap)
    if not all(bootstrap[k] for k in ("session_protocol_supported", "wss", "turn_available", "turn_credentials_present")):
        raise RuntimeError("public provider configuration is incompatible")
    processes = []
    logs = []
    reports = {}
    host_snapshots = {}
    player_snapshots = {}
    error = None
    cleanup = {"disconnected": False, "room_expired": False}
    room = None
    matched_ticks = []
    try:
        for role in ("host", "player"):
            env = os.environ.copy()
            env.update(OPEN_BEVY_ONLINE_VERIFY_ROLE=role,
                       OPEN_BEVY_ONLINE_VERIFY_MODE="human-ai",
                       OPEN_BEVY_ONLINE_VERIFY_RUN=args.run_id,
                       OPEN_BEVY_ONLINE_VERIFY_STATUS=str(output / f"{role}.json"),
                       OPEN_BEVY_SIGNALING_URL=args.service,
                       BEVY_ASSET_ROOT=str(ROOT), RUST_LOG="warn")
            log = (output / f"{role}.log").open("w")
            logs.append(log)
            processes.append(subprocess.Popen(
                [str(ROOT / "scripts/native_runner.sh"), str(binary)],
                cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT,
                start_new_session=True))
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            for role, store in (("host", host_snapshots), ("player", player_snapshots)):
                value = read_json(output / f"{role}.json")
                if value is None:
                    continue
                reports[role] = value
                room = value.get("room_code") or room
                for snapshot in (value.get("control") or {}).get("snapshots", []):
                    store[snapshot["tick"]] = snapshot
            if len(reports) == 2 and all(r.get("terminal") for r in reports.values()):
                break
            if any(p.poll() is not None for p in processes):
                raise RuntimeError("a native client exited before terminal proof")
            time.sleep(0.1)
        if len(reports) != 2 or not all(r.get("passed") for r in reports.values()):
            raise RuntimeError("native control proof failed or timed out; inspect local reports")
        if any(r.get("run_id") != args.run_id or not r.get("human_ai_control") or r.get("connected_humans") != 2 for r in reports.values()):
            raise RuntimeError("wrong verification scenario or connected peer count")
        if reports["host"].get("room_code") != reports["player"].get("room_code"):
            raise RuntimeError("native clients joined different rooms")
        host = reports["host"]["control"]
        player = reports["player"]["control"]
        if not host["queue_seen_on_host"] or not host["queue_preserved_on_host"]:
            raise RuntimeError("authoritative host did not retain both queued moves")
        if player["sent_steps"] != 3 or not all(r["control"]["route_completed"] and r["control"]["ai_factory_completed"] and r["control"]["ai_armed_produced"] for r in reports.values()):
            raise RuntimeError("normal orders or AI development were not observed")
        common = sorted(tick for tick in host_snapshots.keys() & player_snapshots.keys()
                        if host_snapshots[tick].get("clock", 0) >= 30.0)
        if len(common) < 3:
            raise RuntimeError("fewer than three same-tick authoritative snapshots observed")
        mismatches = [tick for tick in common if host_snapshots[tick] != player_snapshots[tick]]
        if mismatches:
            raise RuntimeError(f"same-tick snapshot summaries differ at {mismatches[:5]}")
        matched_ticks = common
        atomic_json(output / "snapshot_agreement.json", {
            "matched_ticks": common, "count": len(common),
            "host_queue_only": True, "unit_queues_not_in_snapshot_protocol": True})
    except Exception as exc:
        error = str(exc)
    finally:
        for process in processes:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGINT)
        for process in processes:
            try:
                process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
        for log in logs:
            log.close()
        cleanup["disconnected"] = True
        if room:
            # Public service has no DELETE endpoint. Its normal host-disconnect
            # grace expires this own QA room; validate collection without tokens.
            deadline = time.monotonic() + 65
            while time.monotonic() < deadline:
                try:
                    fetch(f"{args.service}/v1/rooms/bevy-open-rts/4/{room}")
                except urllib.error.HTTPError as exc:
                    if exc.code == 404:
                        cleanup["room_expired"] = True
                        break
                except Exception:
                    pass
                time.sleep(2)
    if room and not cleanup["room_expired"]:
        error = error or "own QA room did not expire after normal host disconnect"
    result = {"passed": error is None, "error": error, "run_id": args.run_id,
              "room_code": room, "binary_sha256": binary_hash,
              "roles": ["host", "player"], "ai_slots": 1,
              "same_tick_snapshot_count": len(matched_ticks),
              "host_queue_only": True, "cleanup": cleanup}
    atomic_json(output / "result.json", result)
    print(json.dumps(result, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
