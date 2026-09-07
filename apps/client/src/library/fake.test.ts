import { describe, expect, it } from "vitest";
import type { ScanRoot } from "../platform/contracts";
import { LibraryAdapterError } from "./contracts";
import type { PublishedFileLocation } from "./contracts";
import { createFakeLibraryScanAdapter } from "./fake";

const rootA: ScanRoot = {
  id: "root-a",
  displayName: "Projects",
  canonicalPath: "C:\\Synthetic\\Music\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const rootB: ScanRoot = {
  id: "root-b",
  displayName: "Projects",
  canonicalPath: "D:\\Synthetic\\Archive\\Projects",
  enabled: true,
  availability: "available",
  lastErrorCode: null,
};

const makeRecord = (
  root: ScanRoot,
  locationId: string,
  relativePath: string,
): PublishedFileLocation => ({
  locationId,
  rootId: root.id,
  rootDisplayName: root.displayName,
  rootCanonicalPath: root.canonicalPath,
  fileName: `${locationId}.flp`,
  relativePath,
  byteSize: "1024",
  modifiedAt: "2026-01-02T03:04:05.123456789Z",
  presence: "present",
});

async function expectErrorCode(
  promise: Promise<unknown>,
  code: LibraryAdapterError["code"],
) {
  await expect(promise).rejects.toMatchObject({ code });
}

describe("fake per-root pages", () => {
  it("never mixes roots and paginates each root from cursor:null", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      pageLimit: 1,
      files: [
        makeRecord(rootA, "a-one", "One.flp"),
        makeRecord(rootA, "a-two", "Two.flp"),
        makeRecord(rootB, "b-one", "One.flp"),
      ],
    });

    const firstA = await adapter.getLibraryPage({
      rootId: rootA.id,
      limit: 1,
      cursor: null,
    });
    expect(firstA.rootId).toBe(rootA.id);
    expect(firstA.records.map((record) => record.locationId)).toEqual([
      "a-one",
    ]);
    expect(firstA.snapshotId.length).toBeGreaterThan(0);
    expect(firstA.nextCursor).not.toBeNull();

    const secondA = await adapter.getLibraryPage({
      rootId: rootA.id,
      limit: 1,
      cursor: firstA.nextCursor,
    });
    expect(secondA.records.map((record) => record.locationId)).toEqual([
      "a-two",
    ]);
    expect(secondA.snapshotId).toBe(firstA.snapshotId);
    expect(secondA.nextCursor).toBeNull();

    const firstB = await adapter.getLibraryPage({
      rootId: rootB.id,
      limit: 10,
      cursor: null,
    });
    expect(firstB.rootId).toBe(rootB.id);
    expect(firstB.records.map((record) => record.locationId)).toEqual([
      "b-one",
    ]);
    expect(firstB.snapshotId).not.toBe(firstA.snapshotId);
  });

  it("rejects a cursor from root A when querying root B as invalid_cursor", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      pageLimit: 1,
      files: [
        makeRecord(rootA, "a-one", "One.flp"),
        makeRecord(rootA, "a-two", "Two.flp"),
      ],
    });
    const firstA = await adapter.getLibraryPage({
      rootId: rootA.id,
      limit: 1,
      cursor: null,
    });
    expect(firstA.nextCursor).not.toBeNull();
    await expectErrorCode(
      adapter.getLibraryPage({
        rootId: rootB.id,
        limit: 1,
        cursor: firstA.nextCursor,
      }),
      "invalid_cursor",
    );
  });

  it("rejects malformed cursors as invalid_cursor", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      files: [makeRecord(rootA, "a-one", "One.flp")],
    });
    for (const cursor of ["bogus", "fake-page-2", "fake-cursor.!!!"]) {
      await expectErrorCode(
        adapter.getLibraryPage({ rootId: rootA.id, limit: 4, cursor }),
        "invalid_cursor",
      );
    }
  });

  it("returns stale_cursor after publication and a complete page from null", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA],
      pageLimit: 1,
      files: [
        makeRecord(rootA, "a-one", "One.flp"),
        makeRecord(rootA, "a-two", "Two.flp"),
      ],
    });
    const first = await adapter.getLibraryPage({
      rootId: rootA.id,
      limit: 1,
      cursor: null,
    });
    expect(first.nextCursor).not.toBeNull();

    adapter.completeScan(rootA.id);

    await expectErrorCode(
      adapter.getLibraryPage({
        rootId: rootA.id,
        limit: 1,
        cursor: first.nextCursor,
      }),
      "stale_cursor",
    );
    const restarted = await adapter.getLibraryPage({
      rootId: rootA.id,
      limit: 1,
      cursor: null,
    });
    expect(restarted.snapshotId).not.toBe(first.snapshotId);
    expect(restarted.records.map((record) => record.locationId)).toEqual([
      "a-one",
    ]);
  });

  it("keeps another root's cursor valid after a sibling publication", async () => {
    const adapter = createFakeLibraryScanAdapter({
      roots: [rootA, rootB],
      pageLimit: 1,
      files: [
        makeRecord(rootA, "a-one", "One.flp"),
        makeRecord(rootB, "b-one", "One.flp"),
        makeRecord(rootB, "b-two", "Two.flp"),
      ],
    });
    const firstB = await adapter.getLibraryPage({
      rootId: rootB.id,
      limit: 1,
      cursor: null,
    });
    adapter.completeScan(rootA.id);
    const secondB = await adapter.getLibraryPage({
      rootId: rootB.id,
      limit: 1,
      cursor: firstB.nextCursor,
    });
    expect(secondB.records.map((record) => record.locationId)).toEqual([
      "b-two",
    ]);
    expect(secondB.snapshotId).toBe(firstB.snapshotId);
  });

  it("reports not_found for an untracked root", async () => {
    const adapter = createFakeLibraryScanAdapter({ roots: [rootA] });
    await expectErrorCode(
      adapter.getLibraryPage({
        rootId: "root-unknown",
        limit: 4,
        cursor: null,
      }),
      "not_found",
    );
  });

  it("keeps jobId with a null runId while queued", async () => {
    const adapter = createFakeLibraryScanAdapter({ roots: [rootA] });
    const started = await adapter.scanNow(rootA.id);
    expect(started.runId).toBeNull();
    const statuses = await adapter.listScanStatuses();
    expect(statuses[0]).toMatchObject({
      jobId: started.jobId,
      runId: null,
      state: "queued",
    });
  });
});
