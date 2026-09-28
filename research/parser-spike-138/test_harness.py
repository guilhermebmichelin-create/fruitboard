"""Transport regressions use disposable child programs, never extra FLPs."""

import hashlib
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import pyflp_probe
import validate
from process_runner import ProbeFailure, run_json


class ProcessTests(unittest.TestCase):
    def child(self, code, **limits):
        return run_json([sys.executable, "-B", "-c", code], **limits)

    def test_valid_json_and_recovery_after_failure(self):
        with self.assertRaisesRegex(ProbeFailure, "^PARSER_PROCESS_FAILED$"):
            self.child("raise SystemExit(3)")
        result, elapsed = self.child('print(\'{"status":"ok"}\')')
        self.assertEqual(result, {"status": "ok"})
        self.assertGreater(elapsed, 0)

    def test_hang_is_terminated(self):
        started = time.monotonic()
        with self.assertRaisesRegex(ProbeFailure, "^PARSER_TIMEOUT$"):
            self.child("import time; time.sleep(60)", timeout_seconds=0.25)
        self.assertLess(time.monotonic() - started, 5)

    def test_stdout_and_stderr_floods_are_terminated(self):
        for stream in ("stdout", "stderr"):
            with self.subTest(stream=stream):
                with self.assertRaisesRegex(ProbeFailure, "^PARSER_OUTPUT_LIMIT$"):
                    self.child(f"import sys\nwhile True: sys.{stream}.buffer.write(b'x'*4096); sys.{stream}.flush()",
                               stdout_limit=1024, stderr_limit=1024)

    def test_invalid_response_never_leaks_child_text(self):
        for code in ("print('private diagnostic')", "print('[]')",
                     "import sys; sys.stdout.buffer.write(b'\\xff')"):
            with self.subTest(code=code):
                with self.assertRaisesRegex(ProbeFailure, "^PARSER_INVALID_JSON$"):
                    self.child(code)

    def test_both_pipes_are_drained_without_deadlock(self):
        result, _ = self.child("import sys; sys.stderr.write('x'*60000); print('{}')")
        self.assertEqual(result, {})

    def test_input_hash_checked_even_after_process_failure(self):
        # This is a plain test file, never parsed as FLP or added to the corpus.
        with tempfile.TemporaryDirectory(prefix="fruitboard-harness-") as folder:
            path = Path(folder) / "input.txt"
            path.write_bytes(b"before")
            expected = hashlib.sha256(b"before").hexdigest()

            def corrupt_then_fail(*args):
                path.write_bytes(b"after")
                raise ProbeFailure("PARSER_TIMEOUT")

            with patch.object(validate, "run", side_effect=corrupt_then_fail):
                with self.assertRaisesRegex(ValueError, "input changed"):
                    validate.measure(Path("unused"), path, expected)
            path.write_bytes(b"before")
            with patch.object(pyflp_probe, "run_json", side_effect=corrupt_then_fail):
                with self.assertRaisesRegex(ValueError, "input changed"):
                    pyflp_probe.measure(path, expected)

    def test_pyflp_retains_version_and_raw_sample_text(self):
        class VersionObject:
            def __str__(self):
                return "26.1.0.5530"

        version = SimpleNamespace(version=VersionObject())
        self.assertEqual(pyflp_probe.get_field(version, "version", str),
                         {"status": "extracted", "value": "26.1.0.5530"})
        raw = "relative//samples/../Synth.wav"
        channel = SimpleNamespace(events=SimpleNamespace(
            ids={196}, first=lambda _: SimpleNamespace(value=raw)))
        self.assertEqual(pyflp_probe.raw_sample_reference(channel),
                         {"status": "extracted", "value": raw})

    def test_f12_requires_the_preregistered_positive_sample_reference(self):
        expected = validate.CORPUS["FIX-FL2026-SAMPLE.flp"]
        observed = {
            "outcome": "complete", "diagnostics": [],
            "savedVersion": {"status": "extracted", "value": expected[1]},
            "baseTempoBpm": {"status": "extracted", "value": expected[2]},
            "channelNames": {"status": "extracted", "value": [expected[3]]},
            "sampleReferences": {"status": "unavailable"},
        }
        self.assertFalse(validate.compare("FIX-FL2026-SAMPLE.flp", observed, expected)["sampleReferences"])
        observed["sampleReferences"] = {
            "status": "extracted", "value": [validate.F12_SAMPLE_REFERENCE],
        }
        self.assertTrue(validate.compare("FIX-FL2026-SAMPLE.flp", observed, expected)["sampleReferences"])

    def test_missing_default_sampler_name_requires_labeled_inference(self):
        name = "FIX-FL2025-MIN.flp"
        expected = validate.CORPUS[name]
        observed = {
            "outcome": "complete", "diagnostics": [],
            "savedVersion": {"status": "extracted", "value": expected[1]},
            "baseTempoBpm": {"status": "extracted", "value": expected[2]},
            "channelNames": {"status": "extracted", "value": ["Sampler"]},
            "sampleReferences": {"status": "unavailable"},
        }
        self.assertFalse(validate.compare(name, observed, expected)["channelNames"])
        observed["channelNames"] = {
            "status": "inferred", "value": ["Sampler"],
            "method": "sampler-default-for-known-build", "confidence": "high",
            "items": [{"status": "inferred", "value": "Sampler",
                       "method": "sampler-default-for-known-build", "confidence": "high"}],
        }
        self.assertTrue(validate.compare(name, observed, expected)["channelNames"])

    def test_pyflp_enum_diagnostic_is_opt_in_per_bounded_child(self):
        with tempfile.TemporaryDirectory(prefix="fruitboard-harness-") as folder:
            path = Path(folder) / "input.txt"
            path.write_bytes(b"unchanged")
            expected = hashlib.sha256(b"unchanged").hexdigest()
            commands = []

            def capture(command):
                commands.append(command)
                return {"outcome": "parsed"}, 1.0

            with patch.object(pyflp_probe, "run_json", side_effect=capture):
                pyflp_probe.measure(path, expected)
                pyflp_probe.measure(path, expected, enum_compat=True)
            self.assertNotIn("--enum-compat-diagnostic", commands[0])
            self.assertEqual(commands[1][-1], "--enum-compat-diagnostic")
            self.assertEqual(commands[0][:2], [sys.executable, "-B"])
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),
                             expected)


if __name__ == "__main__":
    unittest.main()
