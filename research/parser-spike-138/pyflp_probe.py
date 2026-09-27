"""Research-only PyFLP fallback comparison for the approved nine files.

Run only in an isolated Python 3.11 environment after the Rust shortfall is
recorded. This script imports PyFLP for parsing; it never calls PyFLP's save API.
"""

import argparse
import importlib.metadata
import json
import platform
import subprocess
import sys
import time
from pathlib import Path

from process_runner import ProbeFailure, run_json
from validate import CORPUS, sha256


def get_field(obj, key, convert=None):
    try:
        value = getattr(obj, key)
    except Exception as exc:
        return {"status": "failed", "reason": type(exc).__name__}
    if value is None:
        return {"status": "unavailable", "reason": "PYFLP_RETURNED_NONE"}
    if convert is not None:
        value = convert(value)
    if isinstance(value, (str, int, float, bool)):
        return {"status": "extracted", "value": value}
    return {"status": "unsupported", "reason": "UNEXPECTED_PYFLP_TYPE"}


def probe(path):
    import pyflp

    started = time.perf_counter_ns()
    try:
        project = pyflp.parse(str(path))
    except Exception as exc:
        return {
            "outcome": "failed",
            "exception": type(exc).__name__,
            "elapsedMs": (time.perf_counter_ns() - started) / 1_000_000,
        }
    result = {
        "outcome": "parsed",
        "savedVersion": get_field(project, "version", str),
        "baseTempoBpm": get_field(project, "tempo"),
    }
    try:
        channels = list(project.channels)
        result["channelNames"] = [get_field(channel, "name") for channel in channels]
        # sample_path returns a pathlib.Path that can normalize the stored text.
        # Read the inert event value directly to preserve the raw reference.
        result["sampleReferences"] = [raw_sample_reference(channel) for channel in channels]
    except Exception as exc:
        result["channelNames"] = {"status": "failed", "reason": type(exc).__name__}
        result["sampleReferences"] = {"status": "failed", "reason": type(exc).__name__}
    result["elapsedMs"] = (time.perf_counter_ns() - started) / 1_000_000
    return result


def raw_sample_reference(channel):
    try:
        if 196 not in channel.events.ids:
            return {"status": "unavailable", "reason": "SAMPLE_REFERENCE_NOT_STORED"}
        value = channel.events.first(196).value
        if value == "":
            return {"status": "unavailable", "reason": "SAMPLE_REFERENCE_NOT_STORED"}
        if isinstance(value, str):
            return {"status": "extracted", "value": value}
        return {"status": "unsupported", "reason": "UNEXPECTED_PYFLP_TYPE"}
    except Exception as exc:
        return {"status": "failed", "reason": type(exc).__name__}


def measure(path, expected_hash):
    if sha256(path) != expected_hash:
        raise ValueError(f"unapproved bytes: {path.name}")
    try:
        result, _ = run_json([sys.executable, "-B", str(Path(__file__).resolve()),
                              "--fixture", path.name])
        return result
    except ProbeFailure as exc:
        return {"outcome": "failed", "code": str(exc)}
    finally:
        if sha256(path) != expected_hash:
            raise ValueError(f"input changed during parse: {path.name}")


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--output", type=Path)
    mode.add_argument("--fixture", choices=sorted(CORPUS))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    corpus = root / "fixtures/parser-corpus"
    if args.fixture:
        path = corpus / args.fixture
        expected_hash = CORPUS[args.fixture][0]
        if path.stat().st_size > 4 * 1024 * 1024 or sha256(path) != expected_hash:
            raise ValueError("unapproved fixture bytes")
        try:
            result = probe(path)
        finally:
            if sha256(path) != expected_hash:
                raise ValueError("fixture changed during parse")
        print(json.dumps(result))
        return
    if sorted(p.name for p in corpus.glob("FIX-*.flp")) != sorted(CORPUS):
        raise ValueError("corpus file list differs from approved nine")
    results = {}
    for name, expected in CORPUS.items():
        path = corpus / name
        before = sha256(path)
        if before != expected[0]:
            raise ValueError(f"unapproved bytes: {name}")
        observed = measure(path, expected[0])
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
