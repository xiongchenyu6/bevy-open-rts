#!/usr/bin/env python3
"""Run a native mine proof, persist the child result, and reject missing evidence."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys

p = argparse.ArgumentParser()
p.add_argument("mode", choices=("create", "resume"))
p.add_argument("--binary", type=Path, required=True)
p.add_argument("--output", type=Path, required=True)
p.add_argument("--evidence", type=Path)
p.add_argument("--log", type=Path, required=True)
p.add_argument("--result", type=Path, required=True)
p.add_argument("--timeout", type=float, default=180, help="maximum child seconds (default180)")
a = p.parse_args()
if not math.isfinite(a.timeout) or a.timeout <= 0:
    p.error("timeout must be positive")
for label, path in (("output", a.output), ("log", a.log), ("result", a.result)):
    if path.exists():
        p.error(f"{label} already exists; use fresh paths to preserve prior proof")
if a.mode == "resume" and a.evidence is None:
    p.error("resume requires --evidence")
command = [str(a.binary.absolute()), "save-mines" if a.mode == "create" else "save-mines-resume"]
if a.evidence is not None:
    command.append(str(a.evidence.resolve()))
command.append(str(a.output.resolve()))
for path in (a.log, a.result):
    path.parent.mkdir(parents=True, exist_ok=True)
observations = a.output / "observations.json"
if observations.exists():
    p.error("output already contains observations; use a fresh output directory")
with a.binary.open("rb") as f:
    binary_hash = hashlib.file_digest(f, "sha256").hexdigest()
timed_out = False
with a.log.open("x") as log:
    child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    try:
        child_code = child.wait(timeout=a.timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        child.terminate()
        try:
            child_code = child.wait(timeout=2)
        except subprocess.TimeoutExpired:
            child.kill()
            child_code = child.wait()
error = None
try:
    data = json.loads(observations.read_text())
    required = ("process_id", "initial", "expected", "layer_key", "mine_key") if a.mode == "create" else (
        "process_id", "expected", "restored", "after", "trigger", "mine_source_remapped", "save_bytes_unchanged")
    if not isinstance(data, dict) or not all(key in data for key in required):
        raise ValueError("observations are missing required mine proof fields")
    if data["process_id"] != child.pid:
        raise ValueError("observations PID does not match the actual child")
    if a.mode == "resume":
        previous = json.loads((a.evidence / "observations.json").read_text())
        if data["process_id"] == previous["process_id"]:
            raise ValueError("resume did not use a fresh process")
        if data["mine_source_remapped"] is not True or data["save_bytes_unchanged"] is not True:
            raise ValueError("mine remap/save-byte checks did not pass")
        if not data["trigger"]["player_shot_from_exact_mine"]["player"]:
            raise ValueError("causal player mine shot evidence is missing")
except (OSError, ValueError, KeyError, TypeError) as exc:
    error = str(exc)
exit_code = 124 if timed_out else child_code if child_code > 0 else 128 - child_code if child_code < 0 else 2 if error else 0
result = {"command": command, "binary_sha256": binary_hash, "child_pid": child.pid,
          "child_returncode": child_code, "observations_schema_valid": error is None,
          "observations_error": error, "timed_out": timed_out, "timeout_seconds": a.timeout, "exit_code": exit_code,
          "xdg_data_home": os.environ.get("XDG_DATA_HOME"), "asset_root": os.environ.get("BEVY_ASSET_ROOT")}
with a.result.open("x") as result_file:
    result_file.write(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
raise SystemExit(exit_code)
