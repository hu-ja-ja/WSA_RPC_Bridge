// Unit tests for the pure parts of pipeline.ts. Run via `mise run pipeline:test`.
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { deflateRawSync } from 'node:zlib';
import { Cause, Effect, Exit, Option, Schedule } from 'effect';
import { ArchiveError, buildAndroidUpdateJson, buildDesktopUpdateJson, buildDownloadGuide, extractNotes, extractZipEntry, parseZip, redactSecrets } from './pipeline.ts';

// ── minimal zip builder (local/central/EOCD, no deps) ──
const enc = new TextEncoder();

const buildZip = (entries: { name: string; data: Buffer; method: 0 | 8 }[]): Buffer => {
  const locals: Buffer[] = [];
  const centrals: Buffer[] = [];
  let offset = 0;
  for (const e of entries) {
    const name = Buffer.from(enc.encode(e.name));
    const comp = e.method === 8 ? deflateRawSync(e.data) : e.data;
    const local = Buffer.alloc(30 + name.length);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(e.method, 8);
    local.writeUInt32LE(comp.length, 18);
    local.writeUInt32LE(e.data.length, 22);
    local.writeUInt16LE(name.length, 26);
    name.copy(local, 30);
    const central = Buffer.alloc(46 + name.length);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(e.method, 10);
    central.writeUInt32LE(comp.length, 20);
    central.writeUInt32LE(e.data.length, 24);
    central.writeUInt16LE(name.length, 28);
    central.writeUInt32LE(offset, 42);
    name.copy(central, 46);
    locals.push(local, comp);
    centrals.push(central);
    offset += local.length + comp.length;
  }
  const cd = Buffer.concat(centrals);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(entries.length, 8);
  eocd.writeUInt16LE(entries.length, 10);
  eocd.writeUInt32LE(cd.length, 12);
  eocd.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, cd, eocd]);
};

const run = <A>(effect: Effect.Effect<A, ArchiveError>): Promise<A> => Effect.runPromise(effect);
const runErrorTag = async (effect: Effect.Effect<unknown, ArchiveError>): Promise<string> => {
  const exit = await Effect.runPromiseExit(effect);
  if (!Exit.isFailure(exit)) throw new Error('expected failure');
  const failure = Cause.findErrorOption(exit.cause);
  if (!Option.isSome(failure)) throw new Error('expected failure');
  return failure.value._tag;
};

describe('parseZip/extractZipEntry', () => {
  it('round-trips stored and deflated entries', async () => {
    const zip = buildZip([
      { name: 'a.txt', data: Buffer.from('hello'), method: 0 },
      { name: 'dir/b.bin', data: Buffer.from(enc.encode('x'.repeat(10000))), method: 8 },
    ]);
    const entries = await run(parseZip(zip));
    assert.deepEqual(
      entries.map((e) => e.name),
      ['a.txt', 'dir/b.bin'],
    );
    assert.deepEqual(await run(extractZipEntry(zip, entries[0])), Buffer.from('hello'));
    assert.deepEqual(await run(extractZipEntry(zip, entries[1])), Buffer.from(enc.encode('x'.repeat(10000))));
  });

  it('rejects truncated input', async () => {
    const zip = buildZip([{ name: 'a.txt', data: Buffer.from('hello'), method: 0 }]);
    assert.equal(await runErrorTag(parseZip(zip.subarray(0, 10))), 'ArchiveError');
    const entries = await run(parseZip(zip));
    const inflated = { ...entries[0], compSize: entries[0].compSize + 100 };
    assert.equal(await runErrorTag(extractZipEntry(zip, inflated)), 'ArchiveError');
    const deflated = buildZip([{ name: 'b.bin', data: Buffer.from(enc.encode('y'.repeat(10000))), method: 8 }]);
    const dEntries = await run(parseZip(deflated));
    // cut 3 bytes off the deflate stream itself (local header is 30 + name bytes)
    const cut = deflated.subarray(0, 30 + Buffer.byteLength('b.bin') + dEntries[0].compSize - 3);
    assert.equal(await runErrorTag(extractZipEntry(cut, dEntries[0])), 'ArchiveError');
  });

  it('rejects unknown compression methods', async () => {
    const zip = buildZip([{ name: 'a.txt', data: Buffer.from('hello'), method: 0 }]);
    const entries = await run(parseZip(zip));
    assert.equal(await runErrorTag(extractZipEntry(zip, { ...entries[0], method: 12 })), 'ArchiveError');
  });
});

describe('extractNotes', () => {
  const changelog = [
    '# Changelog',
    '',
    '## [0.4.1] - 2026-09-10',
    '',
    '### 変更',
    '',
    '- foo',
    '',
    '## [0.4.0] - 2026-09-09',
    '',
    '- bar',
    '',
  ].join('\n');

  it('extracts the matching section without the header', () => {
    assert.equal(extractNotes(changelog, '0.4.1'), '### 変更\n\n- foo');
  });

  it('extracts the last section', () => {
    assert.equal(extractNotes(changelog, '0.4.0'), '- bar');
  });

  it('returns undefined for a missing version', () => {
    assert.equal(extractNotes(changelog, '9.9.9'), undefined);
  });
});

describe('redactSecrets', () => {
  it('masks secret values and leaves the rest intact', () => {
    assert.equal(
      redactSecrets('Command failed: keytool -storepass s3cr3t -alias myalias', ['s3cr3t', 'myalias']),
      'Command failed: keytool -storepass [redacted] -alias [redacted]',
    );
    assert.equal(redactSecrets('nothing secret here', ['s3cr3t']), 'nothing secret here');
  });
});

describe('buildDownloadGuide', () => {
  it('embeds exact names with direct links (ja+en)', () => {
    const guide = buildDownloadGuide({
      version: '0.4.1',
      repo: 'o/r',
      msi: 'a_0.4.1_x64_en-US.msi',
      apk: 'w_0.4.1_android_arm64.apk',
    });
    assert.match(guide, /\[a_0\.4\.1_x64_en-US\.msi\]\(https:\/\/github\.com\/o\/r\/releases\/download\/0\.4\.1\/a_0\.4\.1_x64_en-US\.msi\)/);
    assert.match(guide, /\[w_0\.4\.1_android_arm64\.apk\]\(https:\/\/github\.com\/o\/r\/releases\/download\/0\.4\.1\/w_0\.4\.1_android_arm64\.apk\)/);
    assert.match(guide, /ダウンロード \/ Download/);
  });

  it('omits the android row when no APK was built', () => {
    const guide = buildDownloadGuide({ version: '0.4.1', repo: 'o/r', msi: 'a.msi' });
    assert.doesNotMatch(guide, /Android:/);
    assert.match(guide, /a\.msi/);
  });
});

describe('update-json split', () => {
  it('desktop manifest keeps windows-only with signature+url (Tauri strict parse)', () => {
    const json = buildDesktopUpdateJson({
      version: '0.4.1',
      notes: 'n',
      pubDate: '2026-09-12T00:00:00.000Z',
      signature: 'sig',
      url: 'https://github.com/o/r/releases/download/0.4.1/a.msi',
    }) as { platforms: Record<string, { signature?: unknown; url?: unknown }> };
    assert.deepEqual(Object.keys(json.platforms), ['windows-x86_64']);
    for (const entry of Object.values(json.platforms)) {
      assert.equal(typeof entry.signature, 'string');
      assert.ok((entry.signature as string).length > 0);
      assert.ok((entry.url as string).startsWith('https://'));
    }
    assert.doesNotMatch(JSON.stringify(json), /android-aarch64/);
  });

  it('android manifest carries a top-level url and no platforms key', () => {
    const json = buildAndroidUpdateJson({
      version: '0.4.1',
      notes: 'n',
      pubDate: '2026-09-12T00:00:00.000Z',
      url: 'https://github.com/o/r/releases/download/0.4.1/w.apk',
    }) as Record<string, unknown>;
    assert.equal(json['url'], 'https://github.com/o/r/releases/download/0.4.1/w.apk');
    assert.ok(!('platforms' in json));
  });
});

describe('retry times', () => {
  it('retries up to `times` after the initial attempt', async () => {
    let attempts = 0;
    const alwaysFails = Effect.gen(function* () {
      attempts++;
      return yield* Effect.fail(new ArchiveError({ message: 'boom' }));
    });
    const exit = await Effect.runPromiseExit(
      alwaysFails.pipe(
        Effect.retry({
          schedule: Schedule.spaced(1),
          times: 4,
          while: () => true,
        }),
      ),
    );
    assert.ok(Exit.isFailure(exit));
    assert.equal(attempts, 5);
  });
});
