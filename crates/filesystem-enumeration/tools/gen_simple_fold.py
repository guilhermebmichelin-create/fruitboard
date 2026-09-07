#!/usr/bin/env python3
"""Regenerate src/simple_fold.rs from the vendored Unicode CaseFolding.txt.

Provenance: https://www.unicode.org/Public/17.0.0/ucd/CaseFolding.txt
(Unicode 17.0.0, matching unicode-normalization 0.1.25's UCD version).

Simple case folding uses exactly the C (common) and S (simple) entries;
F (full) and T (Turkic) entries are excluded per the CaseFolding.txt header.
Every C/S mapping in Unicode 17.0.0 targets a single code point (asserted
below), so the table is a flat sorted (source, folded) pair list.

Usage (run from crates/filesystem-enumeration):
    python3 tools/gen_simple_fold.py
"""

from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "CaseFolding.txt"
DEST = HERE.parent / "src" / "simple_fold.rs"

HEADER = """//! Unicode simple case folding table for `LocatorKeyV1` insensitive segments.
//!
//! GENERATED FILE — do not edit by hand. Regenerate with:
//! `python3 tools/gen_simple_fold.py` (run from `crates/filesystem-enumeration`).
//!
//! Provenance: Unicode 17.0.0 `CaseFolding.txt`
//! (https://www.unicode.org/Public/17.0.0/ucd/CaseFolding.txt),
//! matching the UCD version behind `unicode-normalization 0.1.25` used for
//! NFC in this crate. Only status C (common) and S (simple) entries are
//! included; F (full) and T (Turkic) entries are excluded per the data file
//! header, which is exactly Unicode simple case folding. This table is the
//! documented equivalence set for the contract's "Unicode simple case
//! folding of the NFC form" rule (integration contract rev 2, §1.1): two
//! spellings share an `i:` key segment iff their NFC forms agree
//! character-for-character after applying this table.
//!
//! Notable consequences (all covered by unit tests):
//! - ASCII `A-Z` folds to `a-z`.
//! - `µ` (U+00B5) folds to `μ` (U+03BC); `ς` (U+03C2) folds to `σ` (U+03C3).
//! - `ß` (U+00DF) has a full-only mapping and is unchanged by simple folding.
//! - `İ` (U+0130) has a Turkic-only mapping and is unchanged by simple
//!   folding (unlike `str::to_lowercase`, which yields `i` + U+0307).
//! - `ẞ` (U+1E9E) folds to `ß` (U+00DF).

/// Sorted `(source, simple-fold)` code-point pairs (Unicode 17.0.0, C+S).
#[rustfmt::skip]
pub(crate) const SIMPLE_FOLD_TABLE: &[(u32, u32)] = &[
"""


def main() -> None:
    entries: dict[int, int] = {}
    for line in SOURCE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        code, status, mapping = (part.strip() for part in line.split(";")[:3])
        if status not in ("C", "S"):
            continue
        targets = mapping.split()
        assert len(targets) == 1, f"multi-char simple mapping: {line}"
        entries[int(code, 16)] = int(targets[0], 16)
    assert len(entries) > 1400, f"unexpectedly small table: {len(entries)}"
    # Spot-check the documented equivalence set so regeneration drift fails.
    assert entries[0x41] == 0x61, "ASCII A must fold to a"
    assert entries[0x5A] == 0x7A, "ASCII Z must fold to z"
    assert entries[0xB5] == 0x3BC, "micro sign must fold to mu"
    assert entries[0x3C2] == 0x3C3, "final sigma must fold to sigma"
    assert entries[0x1E9E] == 0xDF, "capital sharp s must fold to sharp s"
    assert 0x130 not in entries, "dotted I must have no simple mapping"
    assert 0xDF not in entries, "sharp s must have no simple mapping"

    lines = [HEADER]
    row = []
    for index, (source, folded) in enumerate(sorted(entries.items())):
        row.append(f"(0x{source:04X}, 0x{folded:04X}),")
        if len(row) == 6:
            lines.append("    " + " ".join(row))
            row = []
    if row:
        lines.append("    " + " ".join(row))
    lines.append("];\n")
    DEST.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {DEST} with {len(entries)} entries")


if __name__ == "__main__":
    main()
