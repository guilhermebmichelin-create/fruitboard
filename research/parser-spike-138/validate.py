"""Run the approved #138 corpus against a built research binary, read-only.

The pre-registered expectations below mirror the merged corpus manifest. Results
go only to the caller-supplied private evidence file, never to the repository.
"""

import argparse
import hashlib
import json
import platform
import statistics
import subprocess
from pathlib import Path

from process_runner import ProbeFailure, run_json


CORPUS = {
    "FIX-BASE-MIN.flp": ("d3caba4e6e1a2cd47074b7b6e0f115d9d896e6da2e95705baff0777ef06b30ae", "24.1.0.4225", 120.0, "Sampler", None),
    "FIX-FL2024-A.flp": ("02ed4f6fc4e3f471a7f0619ac31e64049eb34d9eaccc4b6bded15300b5644b72", "24.1.0.4225", 140.0, "Sampler", None),
    "FIX-FL2024-B.flp": ("347aedcb7cc503a3f166bca2fa8e6c03099fc18de958e70a9cd2834c73a9fed7", "24.1.0.4225", 141.0, "Sampler", None),
    "FIX-FL2025-MIN.flp": ("745f3cbb7ec095b8e03ab010ad6463c90e2c3616d4d671b868ef6fdbc9fe5fc5", "25.1.3.4922", 130.0, "Sampler", None),
    "FIX-FL2026-MIN.flp": ("e0471d55032b7dfaa8bbcd496c487147149b9ce18e0b250882cc6f67f47480ee", "26.1.0.5530", 130.0, "Sampler", None),
    "FIX-FL2026-SAMPLE.flp": ("dc11a613e34f2ec4918addf1ec2562d6a2c75ebc8c322f56bcb42d280e607b39", "26.1.0.5530", 137.0, "Fixture Sample A", None),
    "FIX-RB-LIMIT.flp": ("950fb8dbd33b090d5f78c9b3c635a4cc316a3aa36d4fd1d44913674addefa743", None, None, None, "CHANNEL_COUNT_LIMIT"),
    "FIX-RB-MALFORM.flp": ("561fd951e783bcb111980f33ee287a5da928ea2f3770d97fced01823deedbb45", None, None, None, "EVENT_LENGTH_OUT_OF_BOUNDS"),
    "FIX-RB-TRUNC.flp": ("38837f28cf0723434b9a089decc617af8e0069e19b12bd8d6d4cee612e59a9cd", None, None, None, "TRUNCATED_DATA_CHUNK"),
    "FIX-RB-UNKNOWN.flp": ("06c5b2c4cc6c2f2b647cb40bb745d2c2f5d92293aa36daab6e05b62d0ab0e9f6", "24.1.0.4225", 120.0, "Sampler", None),
}

F12_SAMPLE_REFERENCE = r"C:\Users\Public\Documents\FruitboardFixtures\F12\fixture-silence.wav"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(64 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run(binary: Path, *args: str):
    return run_json([str(binary), *args])


def measure(binary: Path, path: Path, expected_hash: str):
    if sha256(path) != expected_hash:
        raise ValueError(f"unapproved bytes: {path.name}")
    try:
        return run(binary, "parse", str(path))
    finally:
        if sha256(path) != expected_hash:
            raise ValueError(f"input changed during parse: {path.name}")


def compare(name, observed, expected):
    _, version, tempo, expected_name, failure = expected
    checks = {}
    if failure is not None:
        checks["failure"] = observed.get("outcome") == "failed" and observed.get("code") == failure
        for key in ("savedVersion", "baseTempoBpm", "channelNames", "sampleReferences"):
            checks[key] = observed[key]["status"] == "failed" and observed[key]["reason"] == failure
        return checks
    checks["savedVersion"] = observed["savedVersion"] == {"status": "extracted", "value": version}
    checks["baseTempoBpm"] = observed["baseTempoBpm"] == {"status": "extracted", "value": tempo}
    checks["channelNames"] = observed["channelNames"] == {"status": "extracted", "value": [expected_name]}
    if name == "FIX-FL2026-SAMPLE.flp":
        checks["sampleReferences"] = observed["sampleReferences"] == {
            "status": "extracted", "value": [F12_SAMPLE_REFERENCE],
        }
    else:
        checks["sampleReferences"] = observed["sampleReferences"]["status"] == "unavailable"
    if name == "FIX-RB-UNKNOWN.flp":
        checks["unknownEvent"] = observed["outcome"] == "partial" and observed["diagnostics"] == [{"code": "UNSUPPORTED_EVENT", "eventId": 255}]
    else:
        checks["unknownEvent"] = observed["outcome"] == "complete" and observed["diagnostics"] == []
    return checks


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    corpus = root / "fixtures/parser-corpus"
    if sorted(p.name for p in corpus.glob("FIX-*.flp")) != sorted(CORPUS):
        raise ValueError("corpus file list differs from approved ten")
    for name, expected in CORPUS.items():
        if sha256(corpus / name) != expected[0]:
            raise ValueError(f"unapproved bytes: {name}")
    descriptor, cold_start_ms = run(args.binary, "describe")
    health, _ = run(args.binary, "health-check")
    warm_start_ms = [run(args.binary, "describe")[1] for _ in range(10)]
    results = {}
    for name, expected in CORPUS.items():
        path = corpus / name
        try:
            observed, first_ms = measure(args.binary, path, expected[0])
            warm_ms = [measure(args.binary, path, expected[0])[1] for _ in range(10)]
        except ProbeFailure as exc:
            results[name] = {
                "sha256": expected[0], "checks": {"process": False},
                "observed": {"outcome": "failed", "code": str(exc)},
            }
            continue
        results[name] = {
            "sha256": expected[0],
            "checks": compare(name, observed, expected),
            "observed": observed,
            "firstProcessMs": first_ms,
            "warmProcessMedianMs": statistics.median(warm_ms),
        }
    report = {
        "sourceHead": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "binarySha256": sha256(args.binary),
        "binaryBytes": args.binary.stat().st_size,
        "host": platform.platform(),
        "descriptor": descriptor,
        "health": health,
        "firstObservedStartMs": cold_start_ms,
        "warmStartMedianMs": statistics.median(warm_start_ms),
        "results": results,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    mismatches = [(name, field) for name, result in results.items() for field, ok in result["checks"].items() if not ok]
    print(json.dumps({"binarySha256": report["binarySha256"], "binaryBytes": report["binaryBytes"], "mismatches": mismatches, "report": str(args.output)}))
    return 2 if mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
