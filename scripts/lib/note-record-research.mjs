// Research observations only. These numeric words are not qualified MIDI fields.
const BUILD = "26.1.0.5530";
const MAX_FILE_BYTES = 64 * 1024 * 1024;
const MAX_RECORDS = 32;
const HEADER_EVENTS = new Set([
  199, 200, 159, 169, 28, 172, 192, 37, 156, 67, 9, 11, 80, 17, 18, 35, 23, 44,
  30, 10, 194, 206, 207, 167, 202, 195, 0, 237, 231, 146, 216,
]);

function reject(code) {
  throw new Error(code);
}

export function inspectNoteRecordCandidates(bytes) {
  if (!Buffer.isBuffer(bytes)) reject("RESEARCH_INPUT_TYPE");
  if (bytes.length > MAX_FILE_BYTES) reject("RESEARCH_FILE_LIMIT");
  if (
    bytes.length < 22 ||
    bytes.toString("ascii", 0, 4) !== "FLhd" ||
    bytes.readUInt32LE(4) !== 6 ||
    bytes.toString("ascii", 14, 18) !== "FLdt" ||
    bytes.readUInt32LE(18) !== bytes.length - 22
  )
    reject("RESEARCH_HEADER_OR_SIZE");
  let cursor = 22;
  let eventCount = 0;
  let version = null;
  let pendingId = null;
  let started = false;
  let closed = false;
  let recordCount = 0;
  const patterns = [];
  const seen = new Set();
  while (cursor < bytes.length) {
    if (++eventCount > 100000) reject("RESEARCH_EVENT_LIMIT");
    const eventOffset = cursor;
    const id = bytes[cursor++];
    let size;
    if (id < 64) size = 1;
    else if (id < 128) size = 2;
    else if (id < 192) size = id === 172 && version === BUILD ? 3 : 4;
    else {
      size = 0;
      for (let shift = 0; ; shift += 7) {
        if (shift > 28 || cursor >= bytes.length)
          reject("RESEARCH_EVENT_LENGTH");
        const part = bytes[cursor++];
        size += (part & 127) * 2 ** shift;
        if (part < 128) break;
      }
    }
    if (size > bytes.length - cursor) reject("RESEARCH_EVENT_BOUNDS");
    if (id !== 213 && size > 2 * 1024 * 1024)
      reject("RESEARCH_EVENT_PAYLOAD_LIMIT");
    const data = bytes.subarray(cursor, cursor + size);
    if (eventCount === 1 && id !== 199) reject("RESEARCH_VERSION_FIRST");
    if (id === 199) {
      if (version !== null) reject("RESEARCH_DUPLICATE_VERSION");
      if (!data.equals(Buffer.from(`${BUILD}\0`, "ascii")))
        reject("RESEARCH_UNVERIFIED_BUILD");
      version = BUILD;
    }
    if (id === 224) {
      if (closed || pendingId === null) reject("RESEARCH_NOTE_BINDING");
      if (seen.has(pendingId)) reject("RESEARCH_DUPLICATE_PATTERN");
      if (size === 0 || size % 24 !== 0) reject("RESEARCH_NOTE_LAYOUT");
      if (recordCount + size / 24 > MAX_RECORDS)
        reject("RESEARCH_RECORD_LIMIT");
      recordCount += size / 24;
      const records = [];
      for (let offset = 0; offset < size; offset += 24) {
        const record = data.subarray(offset, offset + 24);
        records.push({
          ordinal: offset / 24,
          fileOffset: cursor + offset,
          positionWord: record.readUInt32LE(0),
          flagsWord: record.readUInt16LE(4),
          channelWord: record.readUInt16LE(6),
          lengthWord: record.readUInt32LE(8),
          keyWord: record.readUInt16LE(12),
          byte21: record[21],
          recordBytes: [...record],
        });
      }
      patterns.push({ patternId: pendingId, eventOffset, records });
      seen.add(pendingId);
      pendingId = null;
    } else if (id === 65 && !closed) {
      if (pendingId !== null) reject("RESEARCH_NOTE_BINDING");
      pendingId = data.readUInt16LE(0);
      if (pendingId === 0) reject("RESEARCH_PATTERN_ID");
      started = true;
    } else if (started || !HEADER_EVENTS.has(id)) {
      closed = true;
      pendingId = null;
    }
    cursor += size;
  }
  if (version === null) reject("RESEARCH_VERSION_MISSING");
  return {
    interpretation: "Unqualified numeric words; not MIDI or musical semantics",
    savedBuild: version,
    ppqHeader: bytes.readUInt16LE(12),
    eventCount,
    recordCount,
    patterns,
  };
}
