// CI/release glue for GitHub Actions and local repro. Run via `mise run pipeline:<cmd>`.
// Effect-based: expected failures are tagged errors, the entrypoint maps them to exit codes.
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  appendFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { inflateRawSync } from 'node:zlib';
import { Data, Effect, Exit, Schedule } from 'effect';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SRC_TAURI = join(ROOT, 'src-tauri');
const ANDROID_PROJECT = join(SRC_TAURI, 'gen', 'android');
const APK_DIR = join(ANDROID_PROJECT, 'app', 'build', 'outputs', 'apk', 'universal', 'release');
const MSI_DIR = join(SRC_TAURI, 'target', 'release', 'bundle', 'msi');
const FALLBACK_PACKAGE = 'com.wsarpcbridge.app';

// ── errors ─────────────────────────────────────────────────────────
export class DownloadError extends Data.TaggedError('DownloadError')<{
  readonly message: string;
  readonly retryable: boolean;
}> {}
export class ArchiveError extends Data.TaggedError('ArchiveError')<{
  readonly message: string;
}> {}
export class KeystoreError extends Data.TaggedError('KeystoreError')<{
  readonly message: string;
}> {}
export class ArtifactsError extends Data.TaggedError('ArtifactsError')<{
  readonly message: string;
}> {}
export class CargoError extends Data.TaggedError('CargoError')<{
  readonly message: string;
}> {}
type CiError = DownloadError | ArchiveError | KeystoreError | ArtifactsError | CargoError;

// ── helpers ────────────────────────────────────────────────────────
// Direct process spawn (not via shell despite the history of this helper).
// maxBuffer must fit the largest expected output: `cargo metadata` prints
// megabytes of JSON, and node kills the child with SIGTERM past the limit
// (default is ~1MB). That silent death masqueraded as flaky CI for a while.
const MAX_OUTPUT = 64 * 1024 * 1024;
const runCommand = (cmd: string, args: string[], extraEnv: Record<string, string> = {}): Effect.Effect<string, CargoError> =>
  Effect.try({
    try: () => spawnSync(cmd, args, { cwd: ROOT, encoding: 'utf-8', env: { ...process.env, ...extraEnv }, maxBuffer: MAX_OUTPUT }),
    catch: () => new CargoError({ message: `failed to spawn ${cmd}` }),
  }).pipe(
    Effect.flatMap((r) => {
      if (r.status === 0) return Effect.succeed(r.stdout ?? '');
      const how = r.signal ? `killed by signal ${r.signal}` : `exited with code ${r.status}`;
      return Effect.fail(new CargoError({ message: `command failed (${how}): ${cmd} ${args.join(' ')}\n${r.stderr ?? ''}` }));
    }),
  );

const readEnv = (name: string): string | undefined => {
  const v = process.env[name]?.trim();
  return v || undefined;
};

const requiredEnv = (name: string): Effect.Effect<string, ArtifactsError> => {
  const v = readEnv(name);
  return v === undefined
    ? Effect.fail(new ArtifactsError({ message: `${name} is not set` }))
    : Effect.succeed(v);
};

const secretEnv = (name: string): Effect.Effect<string, KeystoreError> => {
  const v = readEnv(name);
  return v === undefined
    ? Effect.fail(new KeystoreError({ message: `${name} is not set` }))
    : Effect.succeed(v);
};

const readText = (path: string): Effect.Effect<string, ArtifactsError> =>
  Effect.try({
    try: () => readFileSync(path, 'utf-8'),
    catch: () => new ArtifactsError({ message: `cannot read ${path}` }),
  });

const writeText = (path: string, data: string, encoding: BufferEncoding = 'utf-8'): Effect.Effect<void, ArtifactsError> =>
  Effect.try({
    try: () => writeFileSync(path, data, encoding),
    catch: () => new ArtifactsError({ message: `cannot write ${path}` }),
  });

const stripV = (raw: string | undefined, usage: string): Effect.Effect<string, ArtifactsError> => {
  const v = (raw ?? '').replace(/^v/, '').trim();
  return v ? Effect.succeed(v) : Effect.fail(new ArtifactsError({ message: `usage: ${usage}` }));
};

const runnerTemp = (): string => process.env['RUNNER_TEMP'] || tmpdir();

// ── android-codegen ────────────────────────────────────────────────
// tauri.settings.gradle / tauri.build.gradle.kts と generated kotlin は
// gitignore されていて fresh checkout には無い。build.rs (tauri-build) が
// TAURI_ANDROID_PROJECT_PATH と DEP_*_ANDROID_LIBRARY_PATH から gradle
// ファイルを生成する。android target でないと DEP_ 変数は出ないので手動で
// 渡し (cargo は未知の DEP_ 変数を build.rs に通す)、kotlin template の
// {{...}} 展開もここで行う。実ビルドでは android target の build.rs が
// 同じ展開を行う。
// NOTE: Tauri/wry の build.rs と処理が重複している。Tauri更新時はここも要確認。
const cmdAndroidCodegen = (): Effect.Effect<void, CargoError | ArtifactsError> =>
  Effect.gen(function* () {
    const output = yield* runCommand('cargo', [
      'metadata',
      '--format-version',
      '1',
      '--manifest-path',
      join(SRC_TAURI, 'Cargo.toml'),
    ]);
    const meta = yield* Effect.try({
      try: () => JSON.parse(output.replace(/^\uFEFF/, '')) as { packages: { name: string; manifest_path: string }[] },
      catch: () => new CargoError({ message: 'cargo metadata output is invalid JSON' }),
    });
    const rootManifest = resolve(SRC_TAURI, 'Cargo.toml');
    const root = meta.packages.find((p) => p.manifest_path === rootManifest);
    if (!root) return yield* Effect.fail(new CargoError({ message: 'root package not found' }));
    const tauri = meta.packages.find((p) => p.name === 'tauri');
    const wry = meta.packages.find((p) => p.name === 'wry');
    if (!tauri || !wry) return yield* Effect.fail(new CargoError({ message: 'tauri/wry package not found' }));
    const conf = (yield* readText(join(SRC_TAURI, 'tauri.conf.json')).pipe(
      Effect.flatMap((t) =>
        Effect.try({
          try: () => JSON.parse(t) as { identifier?: unknown },
          catch: () => new ArtifactsError({ message: 'cannot parse tauri.conf.json' }),
        })
      ),
    ));
    const pkg = typeof conf.identifier === 'string' ? conf.identifier : FALLBACK_PACKAGE;
    const library = root.name.replaceAll('-', '_');
    const generated = join(ANDROID_PROJECT, 'app', 'src', 'main', 'java', ...pkg.split('.'), 'generated');
    yield* Effect.try({
      try: () => mkdirSync(generated, { recursive: true }),
      catch: () => new ArtifactsError({ message: `cannot create ${generated}` }),
    });

    const env: Record<string, string> = { TAURI_ANDROID_PROJECT_PATH: ANDROID_PROJECT };
    for (const p of meta.packages) {
      if (existsSync(join(dirname(p.manifest_path), 'mobile', 'android'))) {
        env[`DEP_${p.name.toUpperCase().replaceAll('-', '_')}_ANDROID_LIBRARY_PATH`] = join(
          dirname(p.manifest_path),
          'mobile',
          'android',
        );
      }
    }
    yield* Effect.try({
      try: () => {
        cpSync(join(dirname(tauri.manifest_path), 'mobile', 'android-codegen'), generated, { recursive: true });
        cpSync(join(dirname(wry.manifest_path), 'src', 'android', 'kotlin'), generated, { recursive: true });
        for (const f of readdirSync(generated)) {
          if (!f.endsWith('.kt') && !f.endsWith('.pro')) continue;
          const path = join(generated, f);
          writeFileSync(
            path,
            readFileSync(path, 'utf-8')
              .replaceAll('{{package}}', pkg)
              .replaceAll('{{package-unescaped}}', pkg)
              .replaceAll('{{library}}', library)
              .replaceAll('{{class-extension}}', '')
              .replaceAll('{{class-init}}', ''),
            'utf-8',
          );
        }
        const verifier = meta.packages.find((p) => p.name === 'rustls-platform-verifier-android');
        if (!verifier) throw new Error('rustls-platform-verifier-android package not found');
        const verifierVersions = join(dirname(verifier.manifest_path), 'maven', 'rustls', 'rustls-platform-verifier');
        const verifierVersion = readdirSync(verifierVersions).find((name) =>
          existsSync(join(verifierVersions, name, `rustls-platform-verifier-${name}.aar`)),
        );
        if (!verifierVersion) throw new Error('rustls platform verifier AAR not found');
        const appLibs = join(ANDROID_PROJECT, 'app', 'libs');
        mkdirSync(appLibs, { recursive: true });
        cpSync(
          join(verifierVersions, verifierVersion, `rustls-platform-verifier-${verifierVersion}.aar`),
          join(appLibs, 'rustls-platform-verifier.aar'),
        );
      },
      catch: () => new ArtifactsError({ message: 'kotlin template expansion failed' }),
    });
    // rust-cache 復元時に build.rs が up-to-date のままにならないよう再実行を強制
    yield* runCommand('cargo', ['clean', '-p', root.name, '--manifest-path', join(SRC_TAURI, 'Cargo.toml')], env);
    yield* runCommand('cargo', ['check', '--manifest-path', join(SRC_TAURI, 'Cargo.toml')], env);
  });

// ── sync-version ───────────────────────────────────────────────────
const cmdSyncVersion = (raw: string | undefined): Effect.Effect<void, ArtifactsError> =>
  Effect.gen(function* () {
    const version = yield* stripV(raw, 'pipeline.ts sync-version <version>');
    const confPath = join(SRC_TAURI, 'tauri.conf.json');
    const conf = (yield* readText(confPath)).replace(/"version":\s*"[^"]*"/, `"version": "${version}"`);
    yield* writeText(confPath, conf);
    const tomlPath = join(SRC_TAURI, 'Cargo.toml');
    const toml = (yield* readText(tomlPath)).replace(/^version = ".*"/m, `version = "${version}"`);
    yield* writeText(tomlPath, toml);
    yield* Effect.logInfo(`synced version ${version}`);
  });

// ── zip ────────────────────────────────────────────────────────────
// Minimal ZIP reader for this CI task (stdlib only; node has no unzip).
// Intentionally supports only what the SDK archive uses:
// stored/deflated entries, standard headers, no ZIP64.
// Not a general-purpose ZIP implementation.
export interface ZipEntry {
  readonly name: string;
  readonly method: number;
  readonly compSize: number;
  readonly localOffset: number;
}

const parseZipUnsafe = (buf: Buffer): ZipEntry[] => {
  const invalid = (message: string): never => {
    throw new ArchiveError({ message });
  };
  if (buf.length < 22) invalid('not a zip archive (too small)');
  let eocd = -1;
  for (let i = buf.length - 22; i >= Math.max(0, buf.length - 65579); i--) {
    if (buf.readUInt32LE(i) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd < 0) invalid('not a zip archive (EOCD not found)');
  const count = buf.readUInt16LE(eocd + 10);
  let p = buf.readUInt32LE(eocd + 16);
  const entries: ZipEntry[] = [];
  for (let i = 0; i < count; i++) {
    if (buf.readUInt32LE(p) !== 0x02014b50) invalid('zip central directory corrupted');
    const nameLen = buf.readUInt16LE(p + 28);
    const extraLen = buf.readUInt16LE(p + 30);
    const commentLen = buf.readUInt16LE(p + 32);
    entries.push({
      name: buf.subarray(p + 46, p + 46 + nameLen).toString('utf-8'),
      method: buf.readUInt16LE(p + 10),
      compSize: buf.readUInt32LE(p + 20),
      localOffset: buf.readUInt32LE(p + 42),
    });
    p += 46 + nameLen + extraLen + commentLen;
  }
  return entries;
};

export const parseZip = (buf: Buffer): Effect.Effect<ZipEntry[], ArchiveError> =>
  Effect.try({
    try: () => parseZipUnsafe(buf),
    catch: (e) => (e instanceof ArchiveError ? e : new ArchiveError({ message: `zip parse failed: ${String(e)}` })),
  });

export const extractZipEntry = (buf: Buffer, entry: ZipEntry): Effect.Effect<Buffer, ArchiveError> =>
  Effect.try({
    try: () => {
      const p = entry.localOffset;
      if (buf.readUInt32LE(p) !== 0x04034b50) {
        throw new ArchiveError({ message: `zip local header missing for ${entry.name}` });
      }
      const dataStart = p + 30 + buf.readUInt16LE(p + 26) + buf.readUInt16LE(p + 28);
      const data = buf.subarray(dataStart, dataStart + entry.compSize);
      if (data.length < entry.compSize) {
        throw new ArchiveError({ message: `truncated entry ${entry.name}` });
      }
      if (entry.method === 0) return Buffer.from(data);
      if (entry.method === 8) return inflateRawSync(data);
      throw new ArchiveError({ message: `unsupported compression method ${entry.method} for ${entry.name}` });
    },
    catch: (e) => (e instanceof ArchiveError ? e : new ArchiveError({ message: `zip extract failed: ${String(e)}` })),
  });

// ── download-sdk ───────────────────────────────────────────────────
// Transient HTTP failures worth retrying in CI (timeouts, rate limits, 5xx).
const retryableStatus = (status: number): boolean =>
  status === 408 || status === 429 || status === 500 || status === 502 || status === 503 || status === 504;

const cmdDownloadSdk = (): Effect.Effect<void, DownloadError | ArchiveError | ArtifactsError> =>
  Effect.gen(function* () {
    const url = readEnv('SDK_URL');
    if (url === undefined) {
      return yield* Effect.fail(new DownloadError({ message: 'SDK_URL is not set', retryable: false }));
    }
    const out = join(ANDROID_PROJECT, 'app', 'libs', 'discord_partner_sdk.aar');
    yield* Effect.try({
      try: () => mkdirSync(dirname(out), { recursive: true }),
      catch: () => new ArtifactsError({ message: `cannot create ${dirname(out)}` }),
    });
    const zip = yield* Effect.tryPromise({
      try: () =>
        fetch(url).then(async (res) => {
          if (!res.ok) {
            throw new DownloadError({
              message: `SDK download failed: HTTP ${res.status}`,
              retryable: retryableStatus(res.status),
            });
          }
          return Buffer.from(await res.arrayBuffer());
        }),
      catch: (e) =>
        e instanceof DownloadError
          ? e
          : new DownloadError({ message: `SDK download failed: ${String(e)}`, retryable: true }),
    }).pipe(
      Effect.retry({
        schedule: Schedule.exponential('1 second', 2),
        times: 4,
        while: (e) => e.retryable,
      }),
    );
    if (!zip.length) return yield* Effect.fail(new ArchiveError({ message: 'SDK download failed (empty body)' }));
    yield* Effect.logInfo(`downloaded ${zip.length} bytes`);
    const wanted = 'discord_social_sdk/lib/release/discord_partner_sdk.aar';
    const all = yield* parseZip(zip);
    const entry = all.find((e) => e.name.replaceAll('\\', '/').endsWith(wanted));
    if (!entry) {
      for (const e of all.slice(0, 20)) yield* Effect.logInfo(`entry: ${e.name}`);
      return yield* Effect.fail(new ArchiveError({ message: `archive layout unexpected: '${wanted}' not found` }));
    }
    const aar = yield* extractZipEntry(zip, entry);
    yield* Effect.try({
      try: () => writeFileSync(out, aar),
      catch: () => new ArtifactsError({ message: `cannot write ${out}` }),
    });
    yield* Effect.logInfo(`extracted ${aar.length} bytes to ${out}`);
    if (aar.length < 10 * 1024 * 1024) {
      return yield* Effect.fail(new ArchiveError({ message: `extracted file too small (${aar.length} bytes)` }));
    }
    if (!(yield* parseZip(aar)).some((e) => e.name === 'AndroidManifest.xml')) {
      return yield* Effect.fail(new ArchiveError({ message: 'archive validation failed: manifest entry missing' }));
    }
    yield* Effect.logInfo('archive ok: manifest entry present');
  });

// ── keystore ───────────────────────────────────────────────────────
const cmdKeystore = (): Effect.Effect<void, KeystoreError> =>
  Effect.gen(function* () {
    const keystore = join(runnerTemp(), 'release.keystore');
    const raw = Buffer.from(yield* secretEnv('ANDROID_KEYSTORE_BASE64'), 'base64');
    // JKS magic or PKCS12 SEQUENCE; garbage base64 fails here instead of at build time
    if (raw.length < 4 || (raw.readUInt32BE(0) !== 0xfeedfeed && raw[0] !== 0x30)) {
      return yield* Effect.fail(new KeystoreError({ message: 'keystore decode failed: not a keystore file (check the secret)' }));
    }
    yield* Effect.try({
      try: () => writeFileSync(keystore, raw),
      catch: () => new KeystoreError({ message: `cannot write ${keystore}` }),
    });
    // A wrong password or corrupt keystore must fail here with a clear
    // message, not later at build time with a cryptic Gradle error.
    const storepass = yield* secretEnv('KEYSTORE_PASSWORD');
    yield* Effect.try({
      try: () =>
        execFileSync('keytool', ['-list', '-keystore', keystore, '-storepass', storepass], {
          encoding: 'utf-8',
          stdio: ['ignore', 'pipe', 'pipe'],
        }),
      catch: (e) => {
        if (e instanceof Error && 'code' in e && e.code === 'ENOENT') {
          return new KeystoreError({ message: 'keytool not found (install a JDK)' });
        }
        return new KeystoreError({
          message: `keystore credentials rejected: ${redactSecrets(String(e), [storepass])}`,
        });
      },
    });
    const githubEnv = readEnv('GITHUB_ENV');
    if (githubEnv === undefined) {
      return yield* Effect.fail(new KeystoreError({ message: 'GITHUB_ENV is not set' }));
    }
    const alias = yield* secretEnv('KEY_ALIAS');
    const keypass = yield* secretEnv('KEY_PASSWORD');
    yield* Effect.try({
      try: () =>
        appendFileSync(
          githubEnv,
          `KEYSTORE_FILE=${keystore}\nKEYSTORE_PASSWORD=${storepass}\nKEY_ALIAS=${alias}\nKEY_PASSWORD=${keypass}\n`,
          'utf-8',
        ),
      catch: () => new KeystoreError({ message: 'cannot append GITHUB_ENV' }),
    });
    yield* Effect.logInfo(`keystore restored to ${keystore}`);
  });

// ── release-notes ──────────────────────────────────────────────────
export const extractNotes = (changelog: string, version: string): string | undefined => {
  const esc = version.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const lines = changelog.split(/\r?\n/);
  const start = lines.findIndex((l) => new RegExp(`^## \\[\\s*${esc}\\s*\\]`).test(l));
  if (start < 0) return undefined;
  let end = lines.findIndex((l, i) => i > start && /^## \[\s*/.test(l));
  if (end < 0) end = lines.length;
  return lines.slice(start + 1, end).join('\n').trim();
};

// ── download-guide ─────────────────────────────────────────────────
// GitHub Release 本文専用。update.json の notes には含めない。
export const buildDownloadGuide = (args: {
  readonly version: string;
  readonly repo: string;
  readonly msi: string;
  readonly apk?: string;
}): string => {
  const base = `https://github.com/${args.repo}/releases/download/${args.version}`;
  const rows = [
    `- Windows: [${args.msi}](${base}/${args.msi}) をダウンロードしてインストールしてください / Download and install \`${args.msi}\`.`,
  ];
  if (args.apk) {
    rows.push(`- Android: [${args.apk}](${base}/${args.apk}) をダウンロードしてインストールしてください / Download and install \`${args.apk}\`.`);
  }
  rows.push(
    '- `.sig` / `.sha256` / `apk-signing-fingerprint.txt` は検証用です。通常は不要です / Verification files, usually not needed. 詳しくはドキュメントを参照してください / See the docs: https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/verification/ https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/en/verification/',
  );
  return `---\n### ダウンロード / Download\n\n${rows.join('\n')}\n`;
};

const cmdReleaseNotes = (raw: string | undefined): Effect.Effect<void, ArtifactsError> =>
  Effect.gen(function* () {
    const version = yield* stripV(raw, 'pipeline.ts release-notes <version>');
    const body =
      extractNotes(yield* readText(join(ROOT, 'CHANGELOG.md')), version) ??
      `Release ${version}\n\nCHANGELOG.md に ## [${version}] の節がありません。`;
    yield* writeText(join(ROOT, '.release-notes.md'), `${body}\n`);
  });

// ── update-json (APK rename 込み) ──────────────────────────────────
// Tauri updater は platforms の全エントリに signature+url を要求する。
// signature なしの android エントリを同居させると Windows で
// `missing field 'signature'` になるため、Android 用は別ファイルに分離する。
export const buildDesktopUpdateJson = (args: {
  readonly version: string;
  readonly notes: string;
  readonly pubDate: string;
  readonly signature: string;
  readonly url: string;
}): Record<string, unknown> => ({
  version: args.version,
  notes: args.notes,
  pub_date: args.pubDate,
  platforms: {
    'windows-x86_64': {
      signature: args.signature,
      url: args.url,
    },
  },
});

export const buildAndroidUpdateJson = (args: {
  readonly version: string;
  readonly notes: string;
  readonly pubDate: string;
  readonly url: string;
}): Record<string, unknown> => ({
  version: args.version,
  notes: args.notes,
  pub_date: args.pubDate,
  url: args.url,
});
const firstFile = (dir: string, ext: string): Effect.Effect<string | undefined, ArtifactsError> =>
  Effect.try({
    try: () => {
      if (!existsSync(dir)) return undefined;
      return readdirSync(dir)
        .filter((f) => f.endsWith(ext))
        .sort()
        .map((f) => join(dir, f))[0];
    },
    catch: () => new ArtifactsError({ message: `cannot list ${dir}` }),
  });

const cmdUpdateJson = (raw: string | undefined): Effect.Effect<void, ArtifactsError> =>
  Effect.gen(function* () {
    const version = yield* stripV(raw, 'pipeline.ts update-json <version>');
    const msi = yield* firstFile(MSI_DIR, '.msi');
    if (!msi) return yield* Effect.fail(new ArtifactsError({ message: 'no release MSI found' }));
    const repo = yield* requiredEnv('GITHUB_REPOSITORY');
    let apkName: string | undefined;
    const apk = yield* firstFile(APK_DIR, '.apk');
    if (apk) {
      apkName = `wsa-rpc-bridge_${version}_android_arm64.apk`;
      if (basename(apk) !== apkName) {
        const dest = join(APK_DIR, apkName);
        yield* Effect.try({
          try: () => renameSync(apk, dest),
          catch: () => new ArtifactsError({ message: `cannot rename ${apk}` }),
        });
        yield* Effect.logInfo(`renamed to ${apkName}`);
      }
    }
    const sigPath = `${msi}.sig`;
    const sig = existsSync(sigPath) ? (yield* readText(sigPath)).trim() : '';
    if (!sig) return yield* Effect.fail(new ArtifactsError({ message: `updater signature missing: ${sigPath}` }));
    const notesPath = join(ROOT, '.release-notes.md');
    const notes = existsSync(notesPath) ? yield* readText(notesPath) : '';
    const pubDate = new Date().toISOString();
    const json = buildDesktopUpdateJson({
      version,
      notes,
      pubDate,
      signature: sig,
      url: `https://github.com/${repo}/releases/download/${version}/${basename(msi)}`,
    });
    const text = `${JSON.stringify(json, null, 2)}\n`;
    yield* writeText(join(ROOT, 'update.json'), text);
    const pages = join(runnerTemp(), 'pages');
    yield* Effect.try({
      try: () => mkdirSync(pages, { recursive: true }),
      catch: () => new ArtifactsError({ message: `cannot create ${pages}` }),
    });
    yield* writeText(join(pages, 'update.json'), text);
    yield* Effect.logInfo(`update.json generated for version ${version}`);
    if (apkName) {
      const androidJson = buildAndroidUpdateJson({
        version,
        notes,
        pubDate,
        url: `https://github.com/${repo}/releases/download/${version}/${apkName}`,
      });
      const androidText = `${JSON.stringify(androidJson, null, 2)}\n`;
      yield* writeText(join(ROOT, 'update-android.json'), androidText);
      yield* writeText(join(pages, 'update-android.json'), androidText);
      yield* Effect.logInfo(`update-android.json generated: ${apkName}`);
    } else {
      yield* Effect.logWarning('no release APK found; skipping update-android.json');
    }
    const guide = buildDownloadGuide({ version, repo, msi: basename(msi), apk: apkName });
    const prev = existsSync(notesPath) ? yield* readText(notesPath) : '';
    yield* writeText(join(ROOT, '.release-notes.md'), `${prev.trimEnd()}\n\n${guide}`);
  });

// ── checksums ──────────────────────────────────────────────────────
// Redacts secret values before they can reach logs (keytool echoes argv in errors).
export const redactSecrets = (text: string, secrets: string[]): string => {
  let out = text;
  for (const s of secrets) {
    if (s) out = out.replaceAll(s, '[redacted]');
  }
  return out;
};

const cmdChecksums = (): Effect.Effect<void, ArtifactsError> =>
  Effect.gen(function* () {
    const files = existsSync(MSI_DIR) ? readdirSync(MSI_DIR).filter((f) => f.endsWith('.msi')).sort() : [];
    if (!files.length) return yield* Effect.fail(new ArtifactsError({ message: 'no release MSI found' }));
    for (const f of files) {
      const hash = createHash('sha256').update(readFileSync(join(MSI_DIR, f))).digest('hex');
      yield* writeText(join(MSI_DIR, `${f}.sha256`), `${hash}  ${f}\n`, 'ascii');
    }
    const apk = yield* firstFile(APK_DIR, '.apk');
    const keystore = process.env['KEYSTORE_FILE'] ?? join(runnerTemp(), 'release.keystore');
    const alias = process.env['KEY_ALIAS'];
    const storepass = process.env['KEYSTORE_PASSWORD'];
    if (!apk || !existsSync(keystore) || !alias || !storepass) {
      yield* Effect.logWarning('no APK or keystore; skipping fingerprint');
      return;
    }
    const result = yield* Effect.try({
      try: () =>
        execFileSync('keytool', ['-list', '-v', '-keystore', keystore, '-alias', alias, '-storepass', storepass], {
          encoding: 'utf-8',
          stdio: ['ignore', 'pipe', 'pipe'],
          maxBuffer: MAX_OUTPUT,
        }),
      catch: (e) => new ArtifactsError({ message: `keytool failed: ${redactSecrets(String(e), [storepass, alias])}` }),
    }).pipe(Effect.result);
    if (result._tag === 'Failure') {
      yield* Effect.logWarning(`${result.failure.message}; skipping fingerprint`);
      return;
    }
    const m = result.success.match(/SHA256:\s*([0-9A-Fa-f:]+)/);
    if (!m) {
      yield* Effect.logWarning('keytool fingerprint extraction failed');
      return;
    }
    yield* writeText(
      join(runnerTemp(), 'apk-signing-fingerprint.txt'),
      `Signing certificate SHA-256 fingerprint\n${m[1].trim()}\n`,
      'ascii',
    );
    yield* Effect.logInfo(`fingerprint: ${m[1].trim()}`);
  });

// ── dispatch ───────────────────────────────────────────────────────
const USAGE = 'usage: pipeline.ts <android-codegen|sync-version|download-sdk|keystore|release-notes|update-json|checksums>';

const program = (argv: string[]): Effect.Effect<void, CiError> => {
  const [cmd, ...rest] = argv;
  switch (cmd) {
    case 'android-codegen':
      return cmdAndroidCodegen();
    case 'sync-version':
      return cmdSyncVersion(rest[0]);
    case 'download-sdk':
      return cmdDownloadSdk();
    case 'keystore':
      return cmdKeystore();
    case 'release-notes':
      return cmdReleaseNotes(rest[0]);
    case 'update-json':
      return cmdUpdateJson(rest[0]);
    case 'checksums':
      return cmdChecksums();
    default:
      return Effect.fail(new ArtifactsError({ message: USAGE }));
  }
};

const main = (argv: string[]): Effect.Effect<void, CiError> =>
  program(argv).pipe(Effect.tapError((e) => Effect.sync(() => console.error(`error: ${e.message}`))));

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const exit = await Effect.runPromiseExit(main(process.argv.slice(2)));
  if (!Exit.isSuccess(exit)) process.exitCode = 1;
}
