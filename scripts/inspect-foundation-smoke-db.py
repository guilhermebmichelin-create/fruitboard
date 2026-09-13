"""Read-only summary for installed Foundation Smoke evidence databases.

The inspector never opens a database for writing. It reports the durable
ledger, staging fences, committed location counts/digests, and the SQLite
integrity result without including lease tokens or absolute machine paths.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--database", required=True)
    parser.add_argument("--journey-root", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args()


def rows(connection: sqlite3.Connection, query: str, parameters: tuple[Any, ...] = ()) -> list[dict[str, Any]]:
    cursor = connection.execute(query, parameters)
    names = [column[0] for column in cursor.description]
    return [dict(zip(names, row, strict=True)) for row in cursor.fetchall()]


def relative_path(value: str | None, journey_root: Path) -> str | None:
    if value is None:
        return None
    try:
        candidate = Path(value)
        return candidate.relative_to(journey_root).as_posix()
    except (ValueError, OSError):
        return "<outside-journey-root>"


def location_digest(connection: sqlite3.Connection, root_id: str) -> tuple[str, list[dict[str, Any]]]:
    records = rows(
        connection,
        """
        SELECT relative_path, locator_key, project_file_id, presence,
               identity_volume_serial, identity_file_id, byte_size
        FROM file_location
        WHERE scan_root_id = ?1
        ORDER BY locator_key COLLATE BINARY, id
        """,
        (root_id,),
    )
    encoded = "\n".join(
        json.dumps(record, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        for record in records
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest(), records


def inspect(database_path: Path, journey_root: Path) -> dict[str, Any]:
    uri = f"file:{database_path.as_posix()}?mode=ro"
    connection = sqlite3.connect(uri, uri=True)
    connection.row_factory = sqlite3.Row
    try:
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
        roots = rows(
            connection,
            """
            SELECT id, display_name, canonical_path, enabled, availability,
                   last_error_code, configuration_revision, generation,
                   last_successful_run_id, last_successful_generation,
                   last_successful_at_ms
            FROM scan_root
            ORDER BY rowid
            """,
        )
        for root in roots:
            root["canonical_path"] = relative_path(root["canonical_path"], journey_root)

        jobs = rows(
            connection,
            """
            SELECT id, scan_root_id, kind, state, retry_chain_id, attempt,
                   max_attempts, not_before_ms, priority, follow_up_requested,
                   cancellation_requested, created_at_ms, updated_at_ms,
                   last_error_code
            FROM scan_job
            ORDER BY created_at_ms, id
            """,
        )
        runs = rows(
            connection,
            """
            SELECT id, scan_job_id, scan_root_id, generation,
                   configuration_revision, retry_chain_id, attempt, state,
                   cancellation_requested, started_at_ms, finished_at_ms,
                   lease_expires_at_ms, outcome, error_code
            FROM scan_run
            ORDER BY started_at_ms, id
            """,
        )
        stages = rows(
            connection,
            """
            SELECT stage.run_id, stage.scan_root_id, stage.state,
                   stage.record_count, stage.path_bytes, stage.created_at_ms,
                   stage.updated_at_ms, stage.published_at_ms,
                   COUNT(observation.id) AS observation_rows
            FROM scan_stage AS stage
            LEFT JOIN scan_stage_observation AS observation
              ON observation.run_id = stage.run_id
            GROUP BY stage.run_id
            ORDER BY stage.created_at_ms, stage.run_id
            """,
        )
        locations: list[dict[str, Any]] = []
        location_summaries: list[dict[str, Any]] = []
        for root in roots:
            digest, records = location_digest(connection, root["id"])
            counts = {"present": 0, "missing": 0}
            for record in records:
                counts[record["presence"]] = counts.get(record["presence"], 0) + 1
            location_summaries.append(
                {
                    "rootId": root["id"],
                    "rowCount": len(records),
                    "presenceCounts": counts,
                    "locationDigest": digest,
                    "records": records,
                }
            )
            if root["display_name"] == "Hardlink Aliases":
                locations.extend(records)

        return {
            "schemaVersion": 1,
            "database": {
                "name": database_path.name,
                "bytes": database_path.stat().st_size,
                "sha256": hashlib.sha256(database_path.read_bytes()).hexdigest(),
                "integrityCheck": integrity,
                "journalMode": connection.execute("PRAGMA journal_mode").fetchone()[0],
            },
            "roots": roots,
            "jobs": jobs,
            "runs": runs,
            "stages": stages,
            "locations": location_summaries,
            "hardlinkLocationIdentity": locations,
        }
    finally:
        connection.close()


def main() -> int:
    arguments = parse_args()
    database = Path(arguments.database).resolve()
    journey_root = Path(arguments.journey_root).resolve()
    output = Path(arguments.output).resolve()
    if not database.is_file():
        raise SystemExit(f"database does not exist: {database}")
    if not journey_root.is_dir():
        raise SystemExit(f"journey root does not exist: {journey_root}")
    result = inspect(database, journey_root)
    output.write_text(json.dumps(result, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(json.dumps(result, ensure_ascii=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
