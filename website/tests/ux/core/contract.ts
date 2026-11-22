// The layout contract (skill § 4, gates 1 and 2) for one page in one state, and the record the
// gate tests and the round command share.
import {mkdirSync, writeFileSync} from 'node:fs';
import path from 'node:path';
import type {Page} from 'playwright/test';
import {overflowProbe, visibilityProbe, type Overflow} from './probes';

export type Required = {name: string; selector: string; where: 'all' | 'docs'; desktopOnly?: boolean};

export type Violation = {gate: 1 | 2 | 3 | 4 | 5; route: string; viewport: string; theme: string; state: string; what: string; detail: string};

export async function gate1(page: Page, at: Omit<Violation, 'gate' | 'what' | 'detail'>): Promise<Violation[]> {
  const found: Overflow[] = await page.evaluate(overflowProbe);
  return found.map((o) => ({gate: 1, ...at, what: o.kind + ' ' + o.path, detail: o.detail}));
}

export async function gate2(page: Page, required: Required[], at: Omit<Violation, 'gate' | 'what' | 'detail'>, absent: Record<string, string>, docs: boolean, desktop: boolean): Promise<{violations: Violation[]; absentSeen: string[]}> {
  const violations: Violation[] = [];
  const absentSeen: string[] = [];
  for (const r of required) {
    if (r.where === 'docs' && !docs) continue;
    if (r.desktopOnly && !desktop) continue;
    const v = await page.evaluate(visibilityProbe, r.selector);
    if (!v.present) {
      if (absent[r.name]) absentSeen.push(r.name);
      else violations.push({gate: 2, ...at, what: r.name, detail: 'absent'});
      continue;
    }
    if (absent[r.name]) violations.push({gate: 2, ...at, what: r.name, detail: `present now: close ${absent[r.name]} and drop it from knownAbsent`});
    const problems = [!v.inViewport && `outside the viewport ${JSON.stringify(v.rect)}`, v.clippedBy && `clipped by ${v.clippedBy}`, v.coveredAt.length > 0 && `covered at ${v.coveredAt.join(', ')}`].filter(Boolean);
    if (problems.length) violations.push({gate: 2, ...at, what: r.name, detail: problems.join('; ')});
  }
  return {violations, absentSeen};
}

export const resultsDir = path.join(__dirname, '..', '..', '..', '.cache', 'ux', 'results');

export function record(kind: string, name: string, data: unknown): void {
  const dir = path.join(resultsDir, kind);
  mkdirSync(dir, {recursive: true});
  writeFileSync(path.join(dir, name.replace(/[^A-Za-z0-9@._-]+/g, '_') + '.json'), JSON.stringify(data, null, 1));
}
