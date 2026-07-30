import { execSync } from 'child_process'
import { readFileSync, writeFileSync, rmSync, mkdirSync } from 'fs'
import { resolve, join } from 'path'

const ROOT = resolve(import.meta.dirname, '..')
const SRC_TAURI = resolve(ROOT, 'src-tauri')
const OUT_DIR = resolve(ROOT, 'src', 'generated')
const OUT_FILE = resolve(OUT_DIR, 'licenses.ts')

// ── helpers ──────────────────────────────────────────────────────────

function readLicenseFile(pkgPath, pkgName) {
  const candidates = [
    join(pkgPath, 'LICENSE'),
    join(pkgPath, 'LICENSE.md'),
    join(pkgPath, 'LICENSE.txt'),
    join(pkgPath, 'LICENCE'),
    join(pkgPath, 'LICENCE.md'),
    join(pkgPath, 'LICENCE.txt'),
    join(pkgPath, 'LICENSE_APACHE-2.0'),
    join(pkgPath, 'LICENSE_MIT'),
  ]
  for (const f of candidates) {
    try {
      return readFileSync(f, 'utf-8').trim()
    } catch { /* next */ }
  }
  console.warn(`  [warn] no LICENSE file for ${pkgName}, using template`)
  return null
}

const SPDX_TEXTS = {
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
}

// ── 1. Rust ──────────────────────────────────────────────────────────

console.log('[1/3] Running cargo-about...')
const tmpDir = resolve(SRC_TAURI, 'target')
mkdirSync(tmpDir, { recursive: true })
const tmpJson = resolve(tmpDir, '.cargo-about-tmp.json')
execSync('cargo about generate --format json -o ' + tmpJson, {
  cwd: SRC_TAURI,
  encoding: 'utf-8',
  timeout: 300_000,
})
const rustRaw = JSON.parse(readFileSync(tmpJson, 'utf-8'))
const rustEntries = []
for (const lic of rustRaw.licenses) {
  for (const used of lic.used_by) {
    const c = used.crate
    // skip workspace project itself
    if (c.name === 'wsa-rpc-bridge') continue
    rustEntries.push({
      name: c.name,
      version: c.version,
      copyright: Array.isArray(c.authors) ? c.authors.join(', ') : (c.authors || ''),
      license: lic.name,
      url: c.repository || `https://crates.io/crates/${c.name}`,
      text: lic.text.trim(),
    })
  }
}
console.log(`  -> ${rustEntries.length} Rust crates`)

// ── 2. npm (production only) ────────────────────────────────────────

console.log('[2/3] Reading npm production licenses...')

// get flat list of production package paths (including transitive deps)
const prodPaths = execSync('pnpm ls --prod --parseable --depth=Infinity', {
  cwd: ROOT,
  encoding: 'utf-8',
})
  .trim()
  .split(/\r?\n/)
  .filter(Boolean)
  // skip the root project entry (first line = project root)
  .slice(1)

// get full licenses list from pnpm
const npmRaw = JSON.parse(
  execSync('pnpm licenses list --json', { cwd: ROOT, encoding: 'utf-8' })
)

// build a set of production package identity keys
const prodSet = new Set()
for (const p of prodPaths) {
  const name = p.split(/[\\\/]node_modules[\\\/]/).pop()
  if (name) prodSet.add(name.replace(/\\/g, '/'))
}

const npmEntries = []
for (const [licenseName, packages] of Object.entries(npmRaw)) {
  for (const pkg of packages) {
    const pkgName = pkg.name
    if (!prodSet.has(pkgName)) continue
    const pkgPath = pkg.paths[0]
    const ver = Array.isArray(pkg.versions) ? pkg.versions[0] : pkg.versions
    // try to read actual LICENSE file
    let licText = readLicenseFile(pkgPath, pkgName)
    if (!licText) {
      const spdx = licenseName.replace(/ OR .*$/, '').replace(/ AND .*$/, '')
      if (SPDX_TEXTS[spdx]) {
        licText = SPDX_TEXTS[spdx]
      } else {
        licText = `License: ${licenseName}`
      }
    }
    npmEntries.push({
      name: pkgName,
      version: ver,
      copyright: pkg.author || '',
      license: licenseName,
      url: pkg.homepage || '',
      text: licText,
    })
  }
}

// deduplicate npm entries (same package can appear in multiple license groups)
const seen = new Set()
const npmDeduped = npmEntries.filter(e => {
  const key = `${e.name}@${e.version}`
  if (seen.has(key)) return false
  seen.add(key)
  return true
})
console.log(`  -> ${npmDeduped.length} npm packages`)

// ── 3. Merge & write ──────────────────────────────────────────────

console.log('[3/3] Writing src/generated/licenses.ts...')
mkdirSync(OUT_DIR, { recursive: true })

const allEntries = [...rustEntries, ...npmDeduped]

// sort alphabetically for consistency
allEntries.sort((a, b) => a.name.localeCompare(b.name))

const code = `// Auto-generated by scripts/generate-licenses.mjs
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
`

writeFileSync(OUT_FILE, code, 'utf-8')
console.log(`  -> ${allEntries.length} total entries written to ${OUT_FILE}`)

try { rmSync(tmpJson) } catch { /* best effort */ }
