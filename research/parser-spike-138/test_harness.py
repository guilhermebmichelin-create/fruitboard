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


if __name__ == "__main__":
    unittest.main()
