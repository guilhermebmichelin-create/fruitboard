import {
  closeSync,
  copyFileSync,
  createReadStream,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  realpathSync,
  statfsSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

// Opt-in evidence preparation. Never accepts personal project input or reuses
// existing output; run the explicit build-storage preflight first.
const args = process.argv.slice(2);
if (args.length !== 2 || args[0] !== "--output" || !path.isAbsolute(args[1])) {
  throw new Error("Expected --output <fresh absolute evidence directory>.");
}
const output = path.resolve(args[1]);
if (existsSync(output)) throw new Error("Refuse existing fixture output.");
const parent = realpathSync(path.dirname(output));
const capacity = statfsSync(parent, { bigint: true });
if (capacity.bavail * capacity.bsize - 2n * 1024n ** 3n < 30n * 1024n ** 3n) {
  throw new Error("Fixture output would violate the 30 GiB storage reserve.");
}
const base = path.join(parent, path.basename(output));
const approved = fileURLToPath(
  new URL("../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp", import.meta.url),
);
const original = readFileSync(approved);
if (original.length !== 46_703) throw new Error("Unexpected approved fixture.");
const approvedSourceHash = createHash("sha256").update(original).digest("hex");
async function digest(file) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest("hex");
}

mkdirSync(base);
const inventory = [];
for (const [name, bytes] of [
  ["sample", original.length],
  ["reported", 4_601_596],
  ["25mb", 25_000_000],
  ["64mib", 67_108_864],
]) {
  const directory = path.join(base, name);
  const source = path.join(directory, "source");
  mkdirSync(directory);
  mkdirSync(source);
  const file = path.join(source, "fixture-00.flp");
  const header = Buffer.from(original);
  let event = Buffer.alloc(0);
  if (bytes > original.length) {
    // Extend one opaque plugin-state event, preserving the approved sample's
    // recognized facts. These are size probes, not richer compatibility cases.
    let length = bytes - original.length - 5;
    const encoded = [];
    do {
      const next = length & 127;
      length = Math.floor(length / 128);
      encoded.push(next | (length ? 128 : 0));
    } while (length);
    if (encoded.length !== 4) throw new Error("Unexpected event geometry.");
    event = Buffer.from([213, ...encoded]);
    header.writeUInt32LE(bytes - 22, 18);
  }
  const fd = openSync(file, "wx");
  try {
    writeFileSync(fd, header);
    writeFileSync(fd, event);
    let remaining = bytes - header.length - event.length;
    const filler = Buffer.alloc(64 * 1024, 0xa5);
    while (remaining) {
      const amount = Math.min(remaining, filler.length);
      writeFileSync(fd, filler.subarray(0, amount));
      remaining -= amount;
    }
  } finally {
    closeSync(fd);
  }
  for (let index = 1; index < 12; index++) {
    copyFileSync(
      file,
      path.join(source, `fixture-${String(index).padStart(2, "0")}.flp`),
    );
  }
  const sha256 = await digest(file);
  writeFileSync(
    path.join(directory, "synthetic-manifest.json"),
    JSON.stringify(
      {
        schema: "fruitboard/native-resource-fixture/1",
        case: name,
        bytes,
        copies: 12,
        savedBuild: "26.1.0.5530",
        approvedSourceHash,
        sha256,
      },
      null,
      2,
    ),
  );
  for (let index = 0; index < 12; index++) {
    const relative = path.join(
      name,
      "source",
      `fixture-${String(index).padStart(2, "0")}.flp`,
    );
    if ((await digest(path.join(base, relative))) !== sha256) {
      throw new Error("Fixture copy differs. Preserve failed evidence.");
    }
    inventory.push({ relative, bytes, sha256 });
  }
}
writeFileSync(
  path.join(base, "fixture-inventory-before.json"),
  JSON.stringify(inventory, null, 2),
);
if ((await digest(approved)) !== approvedSourceHash)
  throw new Error("Approved source changed.");
console.log(
  `Prepared ${inventory.length} independent synthetic copies; source unchanged.`,
);
