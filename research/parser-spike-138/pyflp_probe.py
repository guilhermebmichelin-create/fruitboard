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


def install_enum_compat_diagnostic():
    """Let Python 3.11 reach PyFLP's existing missing-ID hook in this child only.

    Stable PyFLP's EventEnum has no members. Python's Enum constructor rejects
    any value before calling its _missing_ hook unless the member map is
    nonempty. The out-of-range marker is never an FLP event ID. This opt-in
    diagnostic does not change installed package files or production code.
    """
    from pyflp._events import EventEnum

    if EventEnum.__members__:
        raise RuntimeError("unexpected PyFLP EventEnum state")
    marker = int.__new__(EventEnum, 256)
    marker._name_ = "__research_compat__"
    marker._value_ = 256
    marker.type = None
    EventEnum._member_map_["__research_compat__"] = marker


def probe(path, *, enum_compat=False):
    import pyflp

    if enum_compat:
        install_enum_compat_diagnostic()
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


def measure(path, expected_hash, *, enum_compat=False):
    if sha256(path) != expected_hash:
        raise ValueError(f"unapproved bytes: {path.name}")
    try:
        command = [sys.executable, "-B", str(Path(__file__).resolve()),
                   "--fixture", path.name]
        if enum_compat:
            command.append("--enum-compat-diagnostic")
        result, _ = run_json(command)
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
    parser.add_argument("--enum-compat-diagnostic", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    corpus = root / "fixtures/parser-corpus"
    if args.fixture:
        path = corpus / args.fixture
        expected_hash = CORPUS[args.fixture][0]
        if path.stat().st_size > 4 * 1024 * 1024 or sha256(path) != expected_hash:
            raise ValueError("unapproved fixture bytes")
        try:
            result = probe(path, enum_compat=args.enum_compat_diagnostic)
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
        observed = measure(path, expected[0],
                           enum_compat=args.enum_compat_diagnostic)
        after = sha256(path)
        if after != before:
            raise ValueError(f"input changed during parse: {name}")
        results[name] = {"sha256": before, "observed": observed, "unchanged": True}
    packages = ["pyflp", "construct-typing", "construct", "sortedcontainers", "typing_extensions", "arrow", "python-dateutil", "six", "tzdata"]
    report = {
        "sourceHead": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "python": platform.python_version(),
        "packages": {name: importlib.metadata.version(name) for name in packages},
        "enumCompatDiagnostic": args.enum_compat_diagnostic,
        "results": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = {name: value["observed"].get("exception", value["observed"]["outcome"]) for name, value in results.items()}
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    main()
