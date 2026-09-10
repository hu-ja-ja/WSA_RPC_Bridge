// Unit tests for the pure parts of generate-licenses.ts.
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  buildCargoEntries,
  dedupeByNameVersion,
  parseProdSet,
  readLicenseFile,
  selectProdPackages,
  sortEntries,
  templateText,
} from './generate-licenses.ts';
import type { LicenseEntry } from './generate-licenses.ts';

const entry = (name: string, version = '1.0.0'): LicenseEntry => ({
  name,
  version,
  copyright: '',
  license: 'MIT',
  url: '',
  text: 'text',
});

describe('templateText', () => {
  it('returns the MIT template for a single license', () => {
    const text = templateText('MIT');
    assert.match(text, /MIT License/);
    assert.doesNotMatch(text, /=====/);
  });

  it('maps every component of OR expressions to a template', () => {
    const text = templateText('MIT OR Apache-2.0');
    assert.match(text, /===== MIT =====/);
    assert.match(text, /===== Apache-2\.0 =====/);
    assert.match(text, /MIT License/);
    assert.match(text, /Apache License/);
  });

  it('handles parenthesized expressions', () => {
    assert.equal(templateText('(MIT OR Apache-2.0)'), templateText('MIT OR Apache-2.0'));
  });

  it('falls back to a placeholder for unknown licenses', () => {
    assert.equal(templateText('Foo-1.0'), 'License: Foo-1.0');
  });
});

describe('parseProdSet', () => {
  it('extracts package names and skips the project root', () => {
    const stdout = [
      '/app',
      '/app/node_modules/foo',
      '/app/node_modules/foo/node_modules/bar',
      '/app/node_modules/@scope/baz',
      '',
    ].join('\n');
    assert.deepEqual([...parseProdSet(stdout)].sort(), ['@scope/baz', 'bar', 'foo']);
  });

  it('handles windows separators', () => {
    const stdout = ['C:\\app', 'C:\\app\\node_modules\\win-pkg'].join('\r\n');
    assert.deepEqual([...parseProdSet(stdout)], ['win-pkg']);
  });
});

describe('selectProdPackages', () => {
  it('keeps only production packages with a readable path', () => {
    const raw = {
      MIT: [
        { name: 'foo', versions: ['1.0.0', '0.9.0'], author: 'F', homepage: 'https://foo', paths: ['/app/node_modules/foo'] },
        { name: 'dev-only', versions: '9.9.9', paths: ['/app/node_modules/dev-only'] },
        { name: 'no-path', versions: '1.0.0', paths: [] },
      ],
    };
    const picks = selectProdPackages(raw, new Set(['foo']));
    assert.equal(picks.length, 1);
    assert.deepEqual(picks[0], {
      license: 'MIT',
      name: 'foo',
      version: '1.0.0',
      copyright: 'F',
      url: 'https://foo',
      pkgPath: '/app/node_modules/foo',
    });
  });

  it('returns empty for invalid input', () => {
    assert.deepEqual(selectProdPackages(null, new Set(['foo'])), []);
    assert.deepEqual(selectProdPackages({ MIT: 'nope' }, new Set(['foo'])), []);
  });
});

describe('buildCargoEntries', () => {
  it('flattens licenses, skips the workspace crate, and joins authors', () => {
    const raw = {
      licenses: [
        {
          name: 'MIT',
          text: '  license text  ',
          used_by: [
            { crate: { name: 'wsa-rpc-bridge', version: '0.0.1', authors: ['Me'] } },
            { crate: { name: 'serde', version: '1.0.0', authors: ['A', 'B'], repository: 'https://example.com/serde' } },
            { crate: { name: 'norepo', version: '2.0.0' } },
          ],
        },
      ],
    };
    const entries = buildCargoEntries(raw);
    assert.equal(entries.length, 2);
    assert.equal(entries[0]?.name, 'serde');
    assert.equal(entries[0]?.copyright, 'A, B');
    assert.equal(entries[0]?.url, 'https://example.com/serde');
    assert.equal(entries[0]?.text, 'license text');
    assert.equal(entries[1]?.url, 'https://crates.io/crates/norepo');
  });

  it('returns empty for invalid input', () => {
    assert.deepEqual(buildCargoEntries(null), []);
    assert.deepEqual(buildCargoEntries({ licenses: [{ name: 'MIT' }] }), []);
  });
});

describe('dedupeByNameVersion/sortEntries', () => {
  it('drops duplicates and keeps distinct versions', () => {
    assert.deepEqual(
      dedupeByNameVersion([entry('a', '1'), entry('a', '1'), entry('a', '2')]).map((e) => e.version),
      ['1', '2'],
    );
  });

  it('sorts by name without mutating the input', () => {
    const input = [entry('b'), entry('a')];
    assert.deepEqual(sortEntries(input).map((e) => e.name), ['a', 'b']);
    assert.deepEqual(input.map((e) => e.name), ['b', 'a']);
  });
});

describe('readLicenseFile', () => {
  const silenceWarnings = (): (() => void) => {
    const orig = console.warn;
    console.warn = () => {};
    return () => {
      console.warn = orig;
    };
  };

  it('reads a plain LICENSE file', () => {
    const dir = mkdtempSync(join(tmpdir(), 'lic-'));
    writeFileSync(join(dir, 'LICENSE'), '  hello  ', 'utf-8');
    assert.equal(readLicenseFile(dir, 'pkg'), 'hello');
  });

  it('combines dual-license files with headers', () => {
    const dir = mkdtempSync(join(tmpdir(), 'lic-'));
    writeFileSync(join(dir, 'LICENSE_MIT'), 'mit', 'utf-8');
    writeFileSync(join(dir, 'LICENSE_APACHE-2.0'), 'apache', 'utf-8');
    const text = readLicenseFile(dir, 'pkg');
    assert.match(text ?? '', /===== MIT =====/);
    assert.match(text ?? '', /===== Apache-2\.0 =====/);
  });

  it('returns null for metadata-only or missing licenses', () => {
    const restore = silenceWarnings();
    try {
      const spdxDir = mkdtempSync(join(tmpdir(), 'lic-'));
      writeFileSync(join(spdxDir, 'LICENSE.spdx'), 'spdx', 'utf-8');
      assert.equal(readLicenseFile(spdxDir, 'pkg'), null);
      assert.equal(readLicenseFile(mkdtempSync(join(tmpdir(), 'lic-')), 'pkg'), null);
    } finally {
      restore();
    }
  });
});
