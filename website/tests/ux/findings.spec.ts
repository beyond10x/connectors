// Skill § 5: every finding record has the header fields, and no S1 or S2 finding is closed without
// a regression asset that exists.
import {existsSync, readdirSync, readFileSync} from 'node:fs';
import path from 'node:path';
import {expect, test} from 'playwright/test';

const dir = path.join(__dirname, '..', '..', 'ux', 'findings');
const fields = ['id', 'title', 'class', 'severity', 'metric', 'status', 'found_by', 'evidence_recording', 'evidence_screenshot', 'evidence_lines', 'regression_asset'];

export function header(text: string): Record<string, string> {
  const block = /^---\n([\s\S]*?)\n---/.exec(text)?.[1] ?? '';
  return Object.fromEntries(block.split('\n').map((l) => /^([a-z_]+):\s*(.*)$/.exec(l)).filter(Boolean).map((m) => [m![1], m![2].trim()]));
}

// An asset is `<file> :: <test title or checklist line>`; the file must exist and hold the title.
export function assetHolds(asset: string, root: string): boolean {
  const [file, title] = asset.split('::').map((s) => s.trim());
  if (!file || !title) return false;
  const at = path.join(root, file);
  return existsSync(at) && readFileSync(at, 'utf8').includes(title.replace(/^"|"$/g, ''));
}

test('every finding record is complete, and no closed S1/S2 finding lacks its regression asset', () => {
  const files = readdirSync(dir).filter((f) => /^UXF-\d{3}\.md$/.test(f));
  expect(files.length).toBeGreaterThan(0);
  const problems: string[] = [];
  for (const f of files) {
    const h = header(readFileSync(path.join(dir, f), 'utf8'));
    for (const k of fields) if (!(k in h)) problems.push(`${f}: no ${k}`);
    if (h.id !== f.replace(/\.md$/, '')) problems.push(`${f}: id ${h.id}`);
    if (!/^S[123]$/.test(h.severity ?? '')) problems.push(`${f}: severity ${h.severity}`);
    if (!/^(layout|cue|affordance|copy|noise|engine|input)$/.test(h.class ?? '')) problems.push(`${f}: class ${h.class}`);
    const closed = /^closed/.test(h.status ?? '');
    if (closed && /^S[12]$/.test(h.severity) && !assetHolds(h.regression_asset ?? '', path.join(__dirname, '..', '..'))) problems.push(`${f}: closed ${h.severity} without a regression asset that exists`);
  }
  expect(problems).toEqual([]);
});

test('break: the findings check refuses a closed S2 record without an asset', () => {
  const h = header('---\nid: UXF-999\nseverity: S2\nstatus: closed\nregression_asset: none\n---\n');
  expect(/^closed/.test(h.status) && /^S[12]$/.test(h.severity) && !assetHolds(h.regression_asset, path.join(__dirname, '..', '..'))).toBe(true);
});
