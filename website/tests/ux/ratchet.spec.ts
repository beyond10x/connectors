// Skill § 6, the ratchet: runs after the gate project and compares this run's worst value per task
// (over the gating viewports and both colour modes) with ux/baseline.json. It fails on a value
// worse than the baseline beyond its tolerance, an improvement not taken into the baseline, a
// missing value, and a baseline made worse than main's without a decision id.
// UX_UPDATE_BASELINE=1 writes the measured values into the baseline instead.
import {execFileSync} from 'node:child_process';
import {existsSync, readdirSync, readFileSync, writeFileSync} from 'node:fs';
import path from 'node:path';
import {expect, test} from 'playwright/test';
import {resultsDir} from './core/contract';

type Entry = {value: number; tolerance: number; decision?: string};
type Baseline = {about: string; scenarios: Record<string, Record<string, Entry>>; tolerances: Record<string, string>};

const file = path.join(__dirname, '..', '..', 'ux', 'baseline.json');
const counts = /^(screen_only\.(actions|looks|steers)|keyboard\.presses|glance_cost\.|completion_miss|false_complete)/;
const tolerance = (k: string, v: number) => (k.startsWith('layout_shift') ? 0.01 : counts.test(k) ? 0 : Math.round(v * 0.05 * 100) / 100);

function measured(): Record<string, Record<string, number>> {
  const out: Record<string, Record<string, number>> = {};
  const read = (kind: string) => {
    const d = path.join(resultsDir, kind);
    return existsSync(d) ? readdirSync(d).map((f) => JSON.parse(readFileSync(path.join(d, f), 'utf8'))) : [];
  };
  const put = (task: string, k: string, v: number) => {
    out[task] ??= {};
    out[task][k] = Math.max(out[task][k] ?? -Infinity, v);
  };
  for (const r of read('screen-only')) for (const [k, v] of Object.entries(r.metrics as Record<string, number>)) put(r.task, k, v);
  for (const r of read('keyboard')) put(r.task, 'keyboard.presses', r.presses);
  return out;
}

function onMain(): Baseline | null {
  try {
    const root = execFileSync('git', ['rev-parse', '--show-toplevel'], {cwd: __dirname, encoding: 'utf8'}).trim();
    const rel = path.relative(root, file).split(path.sep).join('/');
    return JSON.parse(execFileSync('git', ['show', `origin/main:${rel}`], {cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore']}));
  } catch {
    return null;
  }
}

export function compare(base: Baseline, now: Record<string, Record<string, number>>, main: Baseline | null): string[] {
  const problems: string[] = [];
  for (const [task, keys] of Object.entries(base.scenarios)) {
    for (const [k, e] of Object.entries(keys)) {
      const v = now[task]?.[k];
      if (v === undefined) problems.push(`${task} ${k}: missing value`);
      else if (v > e.value + e.tolerance + 1e-9) problems.push(`${task} ${k}: ${v} worse than baseline ${e.value} (tolerance ${e.tolerance})`);
      else if (v < e.value - e.tolerance - 1e-9) problems.push(`${task} ${k}: ${v} improves on baseline ${e.value}: take it into the baseline in this merge (UX_UPDATE_BASELINE=1)`);
      const m = main?.scenarios[task]?.[k];
      if (m && e.value > m.value && !e.decision) problems.push(`${task} ${k}: baseline ${e.value} worse than main's ${m.value} without a decision id`);
    }
  }
  for (const [task, keys] of Object.entries(now)) for (const k of Object.keys(keys)) if (!base.scenarios[task]?.[k]) problems.push(`${task} ${k}: missing from the baseline`);
  return problems;
}

test('ratchet against ux/baseline.json', () => {
  const now = measured();
  expect(Object.keys(now).length, 'screen-only results from the gate project').toBeGreaterThan(0);
  if (process.env.UX_UPDATE_BASELINE === '1') {
    const prior: Baseline | null = existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : null;
    const scenarios: Baseline['scenarios'] = {};
    for (const [task, keys] of Object.entries(now))
      scenarios[task] = Object.fromEntries(Object.entries(keys).sort().map(([k, v]) => {
        const decision = prior?.scenarios[task]?.[k]?.decision;
        return [k, {value: v, tolerance: tolerance(k, v), ...(decision ? {decision} : {})}];
      }));
    const base: Baseline = {
      about: prior?.about ?? 'Lower is better. Worst value per task over the gating viewports and both colour modes, from `npm run test:ux`; written by UX_UPDATE_BASELINE=1.',
      scenarios,
      tolerances: {counts: '0', steps: '5 %', layout_shift: '0.01'},
    };
    writeFileSync(file, JSON.stringify(base, null, 2) + '\n');
  }
  expect(existsSync(file), 'ux/baseline.json').toBe(true);
  const problems = compare(JSON.parse(readFileSync(file, 'utf8')), now, onMain());
  expect(problems).toEqual([]);
});

test('break: the ratchet refuses a worse value, an untaken improvement, a missing value and an undecided worse baseline', () => {
  const base: Baseline = {about: '', tolerances: {}, scenarios: {t: {a: {value: 2, tolerance: 0}, b: {value: 2, tolerance: 0}, c: {value: 1, tolerance: 0}, d: {value: 3, tolerance: 0}}}};
  const main: Baseline = {about: '', tolerances: {}, scenarios: {t: {d: {value: 2, tolerance: 0}}}};
  const p = compare(base, {t: {a: 3, b: 1, d: 3}}, main).join('\n');
  expect(p).toMatch(/a: 3 worse/);
  expect(p).toMatch(/b: 1 improves/);
  expect(p).toMatch(/c: missing value/);
  expect(p).toMatch(/d: baseline 3 worse than main's 2 without a decision id/);
});
