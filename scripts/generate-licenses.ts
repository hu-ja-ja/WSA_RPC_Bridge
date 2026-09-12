// Third-party license collector for the in-app licenses tab.
// Run via `pnpm generate-licenses`. Skips work when inputs are unchanged
// unless `--force` is given; `--check` only verifies freshness (for CI).
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SRC_TAURI = join(ROOT, 'src-tauri');
const OUT_DIR = join(ROOT, 'src', 'generated');
const OUT_FILE = join(OUT_DIR, 'licenses.ts');
const HASH_FILE = join(OUT_DIR, '.licenses-hash');
const WORKSPACE_CRATE = 'wsa-rpc-bridge';

const INPUT_FILES = [
  join(ROOT, 'package.json'),
  join(ROOT, 'pnpm-lock.yaml'),
  join(SRC_TAURI, 'Cargo.toml'),
  join(SRC_TAURI, 'Cargo.lock'),
  join(SRC_TAURI, 'about.toml'),
  join(ROOT, 'scripts', 'generate-licenses.ts'),
];

export interface LicenseEntry {
  name: string;
  version: string;
  copyright: string;
  license: string;
  url: string;
  text: string;
}

export interface PnpmPick {
  license: string;
  name: string;
  version: string;
  copyright: string;
  url: string;
  pkgPath: string;
}

// ── small helpers ────────────────────────────────────────────────

const isRecord = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

const firstString = (v: unknown): string => {
  if (typeof v === 'string') return v;
  if (Array.isArray(v)) return v.find((x): x is string => typeof x === 'string') ?? '';
  return '';
};

// Shell-free spawn with a readable error (no string-concatenated commands).
// On Windows, console shims (e.g. pnpm.cmd) need a shell to resolve.
const run = (cmd: string, args: string[], cwd: string, timeoutMs?: number): string => {
  try {
    return execFileSync(cmd, args, {
      cwd,
      encoding: 'utf-8',
      timeout: timeoutMs,
      maxBuffer: 16 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: process.platform === 'win32',
    });
  } catch (e) {
    throw new Error(`${cmd} ${args.join(' ')} failed: ${e instanceof Error ? e.message : String(e)}`);
  }
};

export const hashInputs = (files: string[]): string => {
  const h = createHash('sha256');
  for (const f of files) h.update(readFileSync(f));
  return h.digest('hex');
};

// ── license text resolution ──────────────────────────────────────

const LICENSE_CANDIDATES = ['LICENSE', 'LICENSE.md', 'LICENSE.txt', 'LICENCE', 'LICENCE.md', 'LICENCE.txt'];

// Dual-licensed packages ship one file per license.
const MULTI_LICENSE_FILES: [label: string, file: string][] = [
  ['MIT', 'LICENSE_MIT'],
  ['Apache-2.0', 'LICENSE_APACHE-2.0'],
  ['MIT', 'LICENSE-MIT'],
  ['Apache-2.0', 'LICENSE-APACHE'],
];

export const readLicenseFile = (pkgPath: string, pkgName: string): string | null => {
  for (const f of LICENSE_CANDIDATES) {
    try {
      return readFileSync(join(pkgPath, f), 'utf-8').trim();
    } catch { /* next */ }
  }

  const multi: [label: string, text: string][] = [];
  for (const [label, f] of MULTI_LICENSE_FILES) {
    try {
      multi.push([label, readFileSync(join(pkgPath, f), 'utf-8').trim()]);
    } catch { /* next */ }
  }
  if (multi.length > 0) {
    return multi.length > 1
      ? multi.map(([label, t]) => `===== ${label} =====\n\n${t}`).join('\n\n')
      : (multi[0]?.[1] ?? null);
  }

  // Metadata-only packages carry no text to embed (e.g. SPDX stub files);
  // the caller falls back to the template text below without warning.
  for (const f of ['LICENSE.spdx', 'LICENCE.spdx']) {
    try {
      readFileSync(join(pkgPath, f));
      return null;
    } catch { /* next */ }
  }

  console.warn(`  [warn] no LICENSE file for ${pkgName}, using template`);
  return null;
};

const SPDX_TEXTS: Record<string, string> = {
  'MIT': `MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.`,
  'Apache-2.0': `Apache License

Version 2.0, January 2004

http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1. Definitions.

"License" shall mean the terms and conditions for use, reproduction, and distribution as defined by Sections 1 through 9 of this document.

"Licensor" shall mean the copyright owner or entity authorized by the copyright owner that is granting the License.

"Legal Entity" shall mean the union of the acting entity and all other entities that control, are controlled by, or are under common control with that entity. For the purposes of this definition, "control" means (i) the power, direct or indirect, to cause the direction or management of such entity, whether by contract or otherwise, or (ii) ownership of fifty percent (50%) or more of the outstanding shares, or (iii) beneficial ownership of such entity.

"You" (or "Your") shall mean an individual or Legal Entity exercising permissions granted by this License.

"Source" form shall mean the preferred form for making modifications, including but not limited to software source code, documentation source, and configuration files.

"Object" form shall mean any form resulting from mechanical transformation or translation of a Source form, including but not limited to compiled object code, generated documentation, and conversions to other media types.

"Work" shall mean the work of authorship, whether in Source or Object form, made available under the License, as indicated by a copyright notice that is included in or attached to the work (an example is provided in the Appendix below).

"Derivative Works" shall mean any work, whether in Source or Object form, that is based on (or derived from) the Work and for which the editorial revisions, annotations, elaborations, or other modifications represent, as a whole, an original work of authorship. For the purposes of this License, Derivative Works shall not include works that remain separable from, or merely link (or bind by name) to the interfaces of, the Work and Derivative Works thereof.

"Contribution" shall mean any work of authorship, including the original version of the Work and any modifications or additions to that Work or Derivative Works thereof, that is intentionally submitted to Licensor for inclusion in the Work by the copyright owner or by an individual or Legal Entity authorized to submit on behalf of the copyright owner. For the purposes of this definition, "submitted" means any form of electronic, verbal, or written communication sent to the Licensor or its representatives, including but not limited to communication on electronic mailing lists, source code control systems, and issue tracking systems that are managed by, or on behalf of, the Licensor for the purpose of discussing and improving the Work, but excluding communication that is conspicuously marked or otherwise designated in writing by the copyright owner as "Not a Contribution."

"Contributor" shall mean Licensor and any individual or Legal Entity on behalf of whom a Contribution has been received by Licensor and subsequently incorporated within the Work.

2. Grant of Copyright License. Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable copyright license to reproduce, prepare Derivative Works of, publicly display, publicly perform, sublicense, and distribute the Work and such Derivative Works in Source or Object form.

3. Grant of Patent License. Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable (except as stated in this section) patent license to make, have made, use, offer to sell, sell, import, and otherwise transfer the Work, where such license applies only to those patent claims licensable by such Contributor that are necessarily infringed by their Contribution(s) alone or by combination of their Contribution(s) with the Work to which such Contribution(s) was submitted. If You institute patent litigation against any entity (including a cross-claim or counterclaim in a lawsuit) alleging that the Work or a Contribution incorporated within the Work constitutes direct or contributory patent infringement, then any patent licenses granted to You under this License for that Work shall terminate as of the date such litigation is filed.

4. Redistribution. You may reproduce and distribute copies of the Work or Derivative Works thereof in any medium, with or without modifications, and in Source or Object form, provided that You meet the following conditions:

(a) You must give any other recipients of the Work or Derivative Works a copy of this License; and

(b) You must cause any modified files to carry prominent notices stating that You changed the files; and

(c) You must retain, in the Source form of any Derivative Works that You distribute, all copyright, patent, trademark, and attribution notices from the Source form of the Work, excluding those notices that do not pertain to any part of the Derivative Works; and

(d) If the Work includes a "NOTICE" text file as part of its distribution, then any Derivative Works that You distribute must include a readable copy of the attribution notices contained within such NOTICE file, excluding those notices that do not pertain to any part of the Derivative Works, in at least one of the following places: within a NOTICE text file distributed as part of the Derivative Works; within the Source form or documentation, if provided along with the Derivative Works; or, within a display generated by the Derivative Works, if and wherever such third-party notices normally appear. The contents of the NOTICE file are for informational purposes only and do not modify the License. You may add Your own attribution notices within Derivative Works that You distribute, alongside or as an addendum to the NOTICE text from the Work, provided that such additional attribution notices cannot be construed as modifying the License.

You may add Your own copyright statement to Your modifications and may provide additional or different license terms and conditions for use, reproduction, or distribution of Your modifications, or for any such Derivative Works as a whole, provided Your use, reproduction, and distribution of the Work otherwise complies with the conditions stated in this License.

5. Submission of Contributions. Unless You explicitly state otherwise, any Contribution intentionally submitted for inclusion in the Work by You to the Licensor shall be under the terms and conditions of this License, without any additional terms or conditions. Notwithstanding the above, nothing herein shall supersede or modify the terms of any separate license agreement you may have executed with Licensor regarding such Contributions.

6. Trademarks. This License does not grant permission to use the trade names, trademarks, service marks, or product names of the Licensor, except as required for reasonable and customary use in describing the origin of the Work and reproducing the content of the NOTICE file.

7. Disclaimer of Warranty. Unless required by applicable law or agreed to in writing, Licensor provides the Work (and each Contributor provides its Contributions) on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied, including, without limitation, any warranties or conditions of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A PARTICULAR PURPOSE. You are solely responsible for determining the appropriateness of using or redistributing the Work and assume any risks associated with Your exercise of permissions under this License.

8. Limitation of Liability. In no event and under no legal theory, whether in tort (including negligence), contract, or otherwise, unless required by applicable law (such as deliberate and grossly negligent acts) or agreed to in writing, shall any Contributor be liable to You for damages, including any direct, indirect, special, incidental, or consequential damages of any character arising as a result of this License or out of the use or inability to use the Work (including but not limited to damages for loss of goodwill, work stoppage, computer failure or malfunction, or any and all other commercial damages or losses), even if such Contributor has been advised of the possibility of such damages.

9. Accepting Warranty or Additional Liability. While redistributing the Work or Derivative Works thereof, You may choose to offer, and charge a fee for, acceptance of support, warranty, indemnity, or other liability obligations and/or rights consistent with this License. However, in accepting such obligations, You may act only on Your own behalf and on Your sole responsibility, not on behalf of any other Contributor, and only if You agree to indemnify, defend, and hold each Contributor harmless for any liability incurred by, or claims asserted against, such Contributor by reason of your accepting any such warranty or additional liability.

END OF TERMS AND CONDITIONS

APPENDIX: How to apply the Apache License to your work.

To apply the Apache License to your work, attach the following boilerplate notice, with the fields enclosed by brackets "[]" replaced with your own identifying information. (Don't include the brackets!)  The text should be enclosed in the appropriate comment syntax for the file format. We also recommend that a file or class name and description of purpose be included on the same "printed page" as the copyright notice for easier identification within third-party archives.

Copyright [yyyy] [name of copyright owner]

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.`,
};

// SPDX expressions like "MIT OR Apache-2.0" map every component to a
// template, not just the first one.
export const templateText = (licenseName: string): string => {
  const parts = licenseName
    .split(/[()/,|]+|\s+(?:OR|AND)\s+/)
    .map((p) => p.trim())
    .filter(Boolean);
  const texts = parts.map((p) => SPDX_TEXTS[p] ?? `License: ${p}`);
  return parts.length > 1
    ? texts.map((t, i) => `===== ${parts[i]} =====\n\n${t}`).join('\n\n')
    : (texts[0] ?? `License: ${licenseName}`);
};

// ── pure collectors (no fs/process; unit-tested) ───────────────────

// Production package identities from `pnpm ls --prod --parseable` output.
// Lines without a node_modules segment (e.g. the project root) are skipped
// instead of assuming the root is always the first line.
export const parseProdSet = (stdout: string): Set<string> => {
  const set = new Set<string>();
  for (const line of stdout.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const segs = trimmed.split(/[\\/]node_modules[\\/]/);
    if (segs.length < 2) continue;
    const name = segs.pop()?.replace(/\\/g, '/');
    if (name) set.add(name);
  }
  return set;
};

// One row per production package found in `pnpm licenses list --json`.
// Same package listed under several license groups yields one row per group;
// the caller dedupes after resolving the license text.
export const selectProdPackages = (raw: unknown, prodSet: Set<string>): PnpmPick[] => {
  if (!isRecord(raw)) return [];
  const picks: PnpmPick[] = [];
  for (const [license, packages] of Object.entries(raw)) {
    if (!Array.isArray(packages)) continue;
    for (const pkg of packages) {
      if (!isRecord(pkg)) continue;
      const name = typeof pkg['name'] === 'string' ? pkg['name'] : '';
      if (!name || !prodSet.has(name)) continue;
      const paths = Array.isArray(pkg['paths'])
        ? pkg['paths'].filter((p): p is string => typeof p === 'string')
        : [];
      const pkgPath = paths[0];
      if (!pkgPath) continue;
      picks.push({
        license,
        name,
        version: firstString(pkg['versions']),
        copyright: firstString(pkg['author']),
        url: firstString(pkg['homepage']),
        pkgPath,
      });
    }
  }
  return picks;
};

// Flattened `cargo about generate --format json` output. Skips the workspace
// crate itself and entries without usable license text.
export const buildCargoEntries = (raw: unknown): LicenseEntry[] => {
  if (!isRecord(raw) || !Array.isArray(raw['licenses'])) return [];
  const out: LicenseEntry[] = [];
  for (const lic of raw['licenses']) {
    if (!isRecord(lic)) continue;
    const license = typeof lic['name'] === 'string' ? lic['name'] : '';
    const text = typeof lic['text'] === 'string' ? lic['text'].trim() : '';
    if (!license || !text || !Array.isArray(lic['used_by'])) continue;
    for (const used of lic['used_by']) {
      if (!isRecord(used) || !isRecord(used['crate'])) continue;
      const crate = used['crate'];
      const name = typeof crate['name'] === 'string' ? crate['name'] : '';
      if (!name || name === WORKSPACE_CRATE) continue;
      const version = typeof crate['version'] === 'string' ? crate['version'] : '';
      const authors = crate['authors'];
      const copyright = Array.isArray(authors)
        ? authors.filter((a): a is string => typeof a === 'string').join(', ')
        : typeof authors === 'string'
          ? authors
          : '';
      const repository = typeof crate['repository'] === 'string' && crate['repository'] !== ''
        ? crate['repository']
        : `https://crates.io/crates/${name}`;
      out.push({ name, version, copyright, license, url: repository, text });
    }
  }
  return out;
};

export const dedupeByNameVersion = (entries: LicenseEntry[]): LicenseEntry[] => {
  const seen = new Set<string>();
  return entries.filter((e) => {
    const key = `${e.name}@${e.version}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
};

export const sortEntries = (entries: LicenseEntry[]): LicenseEntry[] =>
  [...entries].sort((a, b) => a.name.localeCompare(b.name));

// ── main ───────────────────────────────────────────────────────────

const USAGE = 'usage: generate-licenses.ts [--force] [--check]';

const parseJson = (text: string, what: string): unknown => {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new Error(`${what} output is invalid JSON`);
  }
};

const main = (): void => {
  const args = process.argv.slice(2);
  const flags = new Set(args);
  if (flags.has('--help') || flags.has('-h')) {
    console.log(USAGE);
    return;
  }
  for (const a of args) {
    if (a !== '--force' && a !== '--check') throw new Error(`unknown argument: ${a}\n${USAGE}`);
  }
  const force = flags.has('--force');
  const check = flags.has('--check');

  const hash = hashInputs(INPUT_FILES);
  const upToDate =
    !force &&
    existsSync(OUT_FILE) &&
    existsSync(HASH_FILE) &&
    readFileSync(HASH_FILE, 'utf-8').trim() === hash.trim();
  if (check) {
    if (!upToDate) throw new Error('licenses out of date, run pnpm generate-licenses');
    console.log('licenses up to date');
    return;
  }
  if (upToDate) {
    console.log('licenses up to date, skipping (inputs unchanged)');
    return;
  }

  // ── 1. Rust ──
  console.log('[1/3] Running cargo-about...');
  const tmpJson = join(tmpdir(), `cargo-about-${process.pid}.json`);
  try {
    run(
      'cargo-about',
      [
        'generate',
        '--format',
        'json',
        '--locked',
        '--target',
        'x86_64-pc-windows-msvc',
        '--target',
        'aarch64-linux-android',
        '-o',
        tmpJson,
      ],
      SRC_TAURI,
      300_000,
    );
    const rustEntries = buildCargoEntries(parseJson(readFileSync(tmpJson, 'utf-8'), 'cargo about'));
    console.log(`  -> ${rustEntries.length} Rust crates`);

    // ── 2. npm (production only) ──
    console.log('[2/3] Reading npm production licenses...');
    const prodSet = parseProdSet(run('pnpm', ['ls', '--prod', '--parseable', '--depth=Infinity'], ROOT));
    const npmRaw = parseJson(run('pnpm', ['licenses', 'list', '--json'], ROOT), 'pnpm licenses');
    const npmEntries = selectProdPackages(npmRaw, prodSet).map((p) => ({
      name: p.name,
      version: p.version,
      copyright: p.copyright,
      license: p.license,
      url: p.url,
      text: readLicenseFile(p.pkgPath, p.name) ?? templateText(p.license),
    }));
    const npmDeduped = dedupeByNameVersion(npmEntries);
    console.log(`  -> ${npmDeduped.length} npm packages`);

    // ── 3. Merge & write ──
    console.log('[3/3] Writing src/generated/licenses.ts...');
    mkdirSync(OUT_DIR, { recursive: true });
    const allEntries = sortEntries([...rustEntries, ...npmDeduped]);
    const code = `// Auto-generated by scripts/generate-licenses.ts
// Do not edit manually.

export interface LicenseEntry {
  name: string
  version: string
  copyright: string
  license: string
  url: string
  text: string
}

export const licenses: LicenseEntry[] = ${JSON.stringify(allEntries, null, 2)}
`;
    writeFileSync(OUT_FILE, code, 'utf-8');
    writeFileSync(HASH_FILE, hash, 'utf-8');
    console.log(`  -> ${allEntries.length} total entries written to ${OUT_FILE}`);
  } finally {
    try {
      rmSync(tmpJson);
    } catch { /* best effort */ }
  }
};

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  try {
    main();
  } catch (e) {
    console.error(`error: ${e instanceof Error ? e.message : String(e)}`);
    process.exitCode = 1;
  }
}
