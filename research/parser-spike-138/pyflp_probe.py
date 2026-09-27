"""Research-only PyFLP fallback comparison for the approved nine files.

Run only in an isolated Python 3.11 environment after the Rust shortfall is
recorded. This script imports PyFLP for parsing; it never calls PyFLP's save API.
"""

import argparse
import importlib.metadata
import json
import platform
import subprocess
import time
from pathlib import Path

import pyflp

from validate import CORPUS, sha256


def get_field(obj, key):
    try:
        value = getattr(obj, key)
    except Exception as exc:
        return {"status": "failed", "reason": type(exc).__name__}
    if value is None:
        return {"status": "unavailable", "reason": "PYFLP_RETURNED_NONE"}
    if isinstance(value, (str, int, float, bool)):
        return {"status": "extracted", "value": value}
    return {"status": "unsupported", "reason": "UNEXPECTED_PYFLP_TYPE"}


def probe(path):
    started = time.perf_counter_ns()
    try:
        project = pyflp.parse(str(path))
    except Exception as exc:
        return {
            "outcome": "failed",
            "exception": type(exc).__name__,
            "message": str(exc)[:200],
            "elapsedMs": (time.perf_counter_ns() - started) / 1_000_000,
        }
    result = {
        "outcome": "parsed",
        "savedVersion": get_field(project, "version"),
        "baseTempoBpm": get_field(project, "tempo"),
    }
    try:
        channels = list(project.channels)
        result["channelNames"] = [get_field(channel, "name") for channel in channels]
        result["sampleReferences"] = [get_field(channel, "sample_path") for channel in channels]
    except Exception as exc:
        result["channelNames"] = {"status": "failed", "reason": type(exc).__name__}
        result["sampleReferences"] = {"status": "failed", "reason": type(exc).__name__}
    result["elapsedMs"] = (time.perf_counter_ns() - started) / 1_000_000
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    corpus = root / "fixtures/parser-corpus"
    if sorted(p.name for p in corpus.glob("FIX-*.flp")) != sorted(CORPUS):
        raise ValueError("corpus file list differs from approved nine")
    results = {}
    for name, expected in CORPUS.items():
        path = corpus / name
        before = sha256(path)
        if before != expected[0]:
            raise ValueError(f"unapproved bytes: {name}")
        observed = probe(path)
        after = sha256(path)
        if after != before:
            raise ValueError(f"input changed during parse: {name}")
        results[name] = {"sha256": before, "observed": observed, "unchanged": True}
    packages = ["pyflp", "construct-typing", "construct", "sortedcontainers", "typing_extensions", "arrow", "python-dateutil", "six", "tzdata"]
    report = {
        "sourceHead": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "python": platform.python_version(),
        "packages": {name: importlib.metadata.version(name) for name in packages},
        "results": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = {name: value["observed"].get("exception", value["observed"]["outcome"]) for name, value in results.items()}
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    main()
