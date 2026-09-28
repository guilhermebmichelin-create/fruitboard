"""Bounded process transport for research probes; never reports child text."""

import json
import math
import subprocess
import threading
import time


TIMEOUT_SECONDS = 10
STDOUT_LIMIT_BYTES = 256 * 1024
STDERR_LIMIT_BYTES = 64 * 1024


class ProbeFailure(Exception):
    """A fixed diagnostic code, without paths or untrusted process output."""


def run_json(command, *, timeout_seconds=TIMEOUT_SECONDS,
             stdout_limit=STDOUT_LIMIT_BYTES, stderr_limit=STDERR_LIMIT_BYTES):
    if not math.isfinite(timeout_seconds) or timeout_seconds <= 0:
        raise ValueError("timeout must be positive and finite")
    if stdout_limit <= 0 or stderr_limit <= 0:
        raise ValueError("output limits must be positive")
    started = time.perf_counter_ns()
    try:
        process = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except OSError:
        raise ProbeFailure("PARSER_START_FAILED") from None

    buffers = [bytearray(), bytearray()]
    failures = [None, None]

    def stop():
        try:
            process.kill()
        except ProcessLookupError:
            pass

    def collect(index, pipe, limit):
        try:
            while chunk := pipe.read1(4096):
                if len(buffers[index]) + len(chunk) > limit:
                    failures[index] = "PARSER_OUTPUT_LIMIT"
                    stop()
                    return
                buffers[index].extend(chunk)
        except OSError:
            failures[index] = "PARSER_PIPE_FAILED"
            stop()
        finally:
            pipe.close()

    readers = [
        threading.Thread(target=collect, args=(0, process.stdout, stdout_limit)),
        threading.Thread(target=collect, args=(1, process.stderr, stderr_limit)),
    ]
    for reader in readers:
        reader.start()
    timed_out = False
    try:
        process.wait(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        timed_out = True
        stop()
    except BaseException:
        stop()
        raise
    finally:
        process.wait()
        for reader in readers:
            reader.join()
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    for failure in failures:
        if failure:
            raise ProbeFailure(failure)
    if timed_out:
        raise ProbeFailure("PARSER_TIMEOUT")
    if process.returncode != 0:
        raise ProbeFailure("PARSER_PROCESS_FAILED")
    try:
        output = json.loads(buffers[0].decode("utf-8"))
        if not isinstance(output, dict):
            raise ValueError("expected object")
    except (UnicodeError, ValueError, RecursionError):
        raise ProbeFailure("PARSER_INVALID_JSON") from None
    return output, elapsed_ms
