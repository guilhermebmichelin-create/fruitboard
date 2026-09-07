import { describe, expect, it } from "vitest";
import {
  LibraryAdapterError,
  MAX_SQLITE_BYTE_SIZE_DECIMAL,
  U64_MAX_DECIMAL,
  byteSizeValidationMessage,
  extractModifiedAtFraction,
  formatByteSizeDecimal,
  groupDecimalDigits,
  validateByteSizeDecimal,
} from "./contracts";

describe("decimal-string byte sizes", () => {
  it("accepts canonical values up to the SQLite bound", () => {
    expect(validateByteSizeDecimal("0")).toEqual({ ok: true, value: 0n });
    expect(validateByteSizeDecimal("1024")).toEqual({
      ok: true,
      value: 1024n,
    });
    expect(validateByteSizeDecimal(MAX_SQLITE_BYTE_SIZE_DECIMAL)).toEqual({
      ok: true,
      value: 9223372036854775807n,
    });
  });

  it("treats u64::MAX as displayable but above the SQLite bound", () => {
    expect(validateByteSizeDecimal(U64_MAX_DECIMAL)).toEqual({
      ok: false,
      reason: "sqlite_bound",
    });
    expect(formatByteSizeDecimal(U64_MAX_DECIMAL)).toBe(
      "18,446,744,073,709,551,615 bytes",
    );
  });

  it("rejects leading-zero, signed, float, empty, and non-string input", () => {
    for (const invalid of [
      "",
      "007",
      "00",
      "+1",
      "-1",
      "1.5",
      "1e3",
      " 12",
      "12 ",
      "0x10",
    ]) {
      expect(validateByteSizeDecimal(invalid)).toEqual({
        ok: false,
        reason: "format",
      });
    }
    expect(validateByteSizeDecimal(1024)).toEqual({
      ok: false,
      reason: "format",
    });
    expect(validateByteSizeDecimal(null)).toEqual({
      ok: false,
      reason: "format",
    });
  });

  it("rejects overflow above u64 and above the SQLite bound distinctly", () => {
    expect(validateByteSizeDecimal("18446744073709551616")).toEqual({
      ok: false,
      reason: "u64_overflow",
    });
    expect(validateByteSizeDecimal("99999999999999999999999999")).toEqual({
      ok: false,
      reason: "u64_overflow",
    });
    expect(validateByteSizeDecimal("9223372036854775808")).toEqual({
      ok: false,
      reason: "sqlite_bound",
    });
    expect(byteSizeValidationMessage("sqlite_bound")).toMatch(
      /9,223,372,036,854,775,807/,
    );
    expect(byteSizeValidationMessage("u64_overflow")).toMatch(
      /18,446,744,073,709,551,615/,
    );
  });

  it("formats with grouping and no Number() precision loss", () => {
    expect(formatByteSizeDecimal("0")).toBe("0 bytes");
    expect(formatByteSizeDecimal("1024")).toBe("1,024 bytes");
    expect(formatByteSizeDecimal("1048576")).toBe("1,048,576 bytes");
    expect(formatByteSizeDecimal(U64_MAX_DECIMAL)).toBe(
      "18,446,744,073,709,551,615 bytes",
    );
    expect(groupDecimalDigits("1000000")).toBe("1,000,000");
  });

  it("throws a safe internal error for non-canonical display input", () => {
    for (const invalid of ["007", "+1", "-1", "1.5", ""]) {
      try {
        formatByteSizeDecimal(invalid);
        expect.unreachable(`expected ${invalid} to throw`);
      } catch (error) {
        expect(error).toBeInstanceOf(LibraryAdapterError);
        expect((error as LibraryAdapterError).code).toBe("internal");
      }
    }
  });
});

describe("RFC 3339 nanosecond timestamps", () => {
  it("retains the fractional nanoseconds without Date conversion", () => {
    expect(extractModifiedAtFraction("2026-01-02T03:04:05.123456789Z")).toBe(
      "123456789",
    );
    expect(extractModifiedAtFraction("2026-01-02T03:04:05.100Z")).toBe("100");
    expect(extractModifiedAtFraction("2026-01-02T03:04:05Z")).toBe("");
    expect(
      extractModifiedAtFraction("2026-01-02T03:04:05.000000001+00:00"),
    ).toBe("000000001");
  });

  it("rejects malformed timestamps and over-precise fractions", () => {
    expect(extractModifiedAtFraction("not-a-date")).toBeNull();
    expect(extractModifiedAtFraction("2026-01-02")).toBeNull();
    expect(extractModifiedAtFraction("1717386245123456789")).toBeNull();
    expect(
      extractModifiedAtFraction("2026-01-02T03:04:05.1234567890Z"),
    ).toBeNull();
  });
});
