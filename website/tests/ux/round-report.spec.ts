// The round command's report (skill § 3.2): reads what measure.spec.ts recorded and writes
// ux/rounds/<UX_ROUND>/report.md, round.json, shots/ (contact sheets per theme, worst moments)
// and ux/rounds/trend.md. Deterministic; no model, no network.
import {existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync} from 'node:fs';
import path from 'node:path';
import {gatingViewports, measuredViewports, site, tasks, themes} from '../../ux/adapter';
import {resultsDir, type Violation} from './core/contract';
import {chooseTheme, settle, test, visit} from './core/fixture';

const round = process.env.UX_ROUND ?? 'R-00';
const ux = path.join(__dirname, '..', '..', 'ux');
const dir = path.join(ux, 'rounds', round);
const shots = path.join(dir, 'shots');
const frames = path.join(__dirname, '..', '..', '.cache', 'ux', 'frames', round);
const viewports = [...gatingViewports, ...measuredViewports];
const gating = new Set(gatingViewports.map((v) => v.name));

const read = (kind: string) => {
  const d = path.join(resultsDir, 'measure', kind);
  return existsSync(d) ? readdirSync(d).sort().map((f) => JSON.parse(readFileSync(path.join(d, f), 'utf8'))) : [];
};

type Run = {task: string; viewport: string; theme: string; done: boolean; metrics: Record<string, number>; steps: {action: string; control?: string; region?: string; glance?: number; words: number; url: string}[]};

function previousRound(): {name: string; json: any} | null {
  const rounds = readdirSync(path.join(ux, 'rounds')).filter((d) => /^R-\d+$/.test(d) && d < round).sort();
  const last = rounds.at(-1);
  if (!last || !existsSync(path.join(ux, 'rounds', last, 'round.json'))) return null;
  return {name: last, json: JSON.parse(readFileSync(path.join(ux, 'rounds', last, 'round.json'), 'utf8'))};
}

function violationGroups(layout: {violations: Violation[]; held: Violation[]}[]): Violation[][] {
  const grouped = new Map<string, Violation[]>();
  for (const v of layout.flatMap((x) => [...x.violations, ...x.held])) {
    const k = `${v.gate}|${v.what.replace(/#[\w-]+/g, '')}|${v.detail.replace(/\d+/g, 'n')}`;
    grouped.set(k, [...(grouped.get(k) ?? []), v]);
  }
  return [...grouped.values()].sort((a, b) => b.length - a.length);
}

// The first instance of a violation group, preferring a gating viewport, and its shot file.
function worstOf(g: Violation[], i: number): {file: string; v: Violation} {
  const v = g.find((x) => gating.has(x.viewport)) ?? g[0];
  const file = `worst-${String(i + 1).padStart(2, '0')}-gate${v.gate}-${v.what.split(' ')[0]}-${v.viewport}-${v.theme}.png`.replace(/[^A-Za-z0-9.-]+/g, '_');
  return {file, v};
}

test('write the round report', () => {
  mkdirSync(shots, {recursive: true});
  const layout: {viewport: string; routes: number; violations: Violation[]; held: Violation[]}[] = read('layout');
  const axeRuns: {viewport: string; theme: string; pages: {route: string; hits: {id: string; impact: string; count: number; nodes: {target: string}[]}[]}[]}[] = read('axe');
  const runs: Run[] = read('screen-only');
  const keys: {task: string; done: boolean; presses: number; focusVisible: boolean}[] = read('keyboard');
  const baseline = existsSync(path.join(ux, 'baseline.json')) ? JSON.parse(readFileSync(path.join(ux, 'baseline.json'), 'utf8')) : null;
  const prev = previousRound();

  // Gate counts per viewport and theme.
  const cell = (vp: string, theme: string) => {
    const l = layout.filter((x) => x.viewport === vp).flatMap((x) => x.violations).filter((v) => v.theme === theme);
    const heldHere = layout.filter((x) => x.viewport === vp).flatMap((x) => x.held).filter((v) => v.theme === theme).length;
    const a = axeRuns.filter((x) => x.viewport === vp && x.theme === theme).flatMap((x) => x.pages);
    const serious = a.flatMap((p) => p.hits.filter((h) => h.impact === 'serious' || h.impact === 'critical').map((h) => ({...h, route: p.route})));
    const lesser = a.flatMap((p) => p.hits.filter((h) => h.impact !== 'serious' && h.impact !== 'critical'));
    const r = runs.filter((x) => x.viewport === vp && x.theme === theme);
    return {
      gate1: l.filter((v) => v.gate === 1).length,
      gate2: l.filter((v) => v.gate === 2).length,
      gate3: r.reduce((s, x) => s + x.metrics.completion_miss + x.metrics.false_complete, 0),
      gate4: r.filter((x) => !x.done).length,
      gate5: serious.reduce((s, h) => s + h.count, 0),
      gate5_rules: [...new Set(serious.map((h) => h.id))],
      axe_lesser: lesser.reduce((s, h) => s + h.count, 0),
      pages: layout.filter((x) => x.viewport === vp).reduce((s, x) => s + x.routes, 0),
      held: heldHere,
      axeRan: a.length > 0,
      botRan: r.length > 0,
    };
  };
  const gates: Record<string, Record<string, ReturnType<typeof cell>>> = {};
  for (const vp of viewports) for (const theme of themes) (gates[vp.name] ??= {})[theme] = cell(vp.name, theme);

  // Task metrics: worst over the gating viewports and both modes (the ratchet's value), and per viewport.
  const worst: Record<string, Record<string, number>> = {};
  for (const r of runs.filter((x) => gating.has(x.viewport)))
    for (const [k, v] of Object.entries(r.metrics)) (worst[r.task] ??= {})[k] = Math.max(worst[r.task]?.[k] ?? -Infinity, v);
  for (const k of keys) (worst[k.task] ??= {})['keyboard.presses'] = k.presses;

  const groups = violationGroups(layout);
  const worstShots = groups.slice(0, 8).map((g, i) => ({...worstOf(g, i), count: g.length}));

  const json = {round, generated_by: 'npm run ux:round', gates, tasks: worst, runs: runs.map((r) => ({task: r.task, viewport: r.viewport, theme: r.theme, done: r.done, metrics: r.metrics})), keyboard: keys, violation_groups: groups.map((g) => ({gate: g[0].gate, what: g[0].what, detail: g[0].detail, count: g.length, viewports: [...new Set(g.map((v) => v.viewport))], themes: [...new Set(g.map((v) => v.theme))], routes: [...new Set(g.map((v) => v.route))].slice(0, 12), route_count: new Set(g.map((v) => v.route)).size})), axe: axeRuns.map((a) => ({viewport: a.viewport, theme: a.theme, hits: a.pages.flatMap((p) => p.hits.map((h) => ({route: p.route, id: h.id, impact: h.impact, nodes: h.count})))}))};
  writeFileSync(path.join(dir, 'round.json'), JSON.stringify(json, null, 1) + '\n');

  // report.md
  const lines: string[] = [];
  lines.push(`# ${round} report`, '', `Written by \`npm run ux:round\` (\`UX_ROUND=${round}\`) from the built site; deterministic, no model, no network. Numbers: \`round.json\`.`, '');
  lines.push('## Gates (violations; bar = 0)', '', '| viewport | theme | gated | pages | 1 no overflow | 2 always visible | 3 completion | 4 screen-only | 5 axe serious+critical nodes | axe moderate/minor |', '|---|---|---|---|---|---|---|---|---|---|');
  for (const vp of viewports) for (const theme of themes) {
    const c = gates[vp.name][theme];
    const d = (ran: boolean, v: string | number) => (ran ? String(v) : '–');
    lines.push(`| ${vp.name} | ${theme} | ${gating.has(vp.name) ? 'yes' : 'measured'} | ${c.pages} | ${c.gate1} | ${c.gate2}${c.held ? ' (+' + c.held + ' held)' : ''} | ${d(c.botRan, c.gate3)} | ${d(c.botRan, c.gate4)} | ${d(c.axeRan, c.gate5 + (c.gate5_rules.length ? ' (' + c.gate5_rules.join(', ') + ')' : ''))} | ${d(c.axeRan, c.axe_lesser)} |`);
  }
  lines.push('', '## Tasks (worst over the gating viewports and both themes; lower is better)', '');
  const metricKeys = ['screen_only.actions', 'screen_only.looks', 'screen_only.steers', 'screen_only.time_to_act_steps.max', 'glance_cost.median', 'glance_cost.p95', 'reading_load.max', 'reading_load.median', 'layout_shift.score', 'keyboard.presses'];
  lines.push('| task | card | ' + metricKeys.join(' | ') + ' |', '|---|---|' + metricKeys.map(() => '---').join('|') + '|');
  for (const t of tasks) {
    const w = worst[t.id] ?? {};
    const b = baseline?.scenarios?.[t.id] ?? {};
    const p = prev?.json?.tasks?.[t.id] ?? {};
    lines.push(`| ${t.id} | ${t.card} | ` + metricKeys.map((k) => `${w[k] ?? '–'}${b[k] ? ' (base ' + b[k].value + ')' : ''}${p[k] !== undefined ? ' (last ' + p[k] + ')' : ''}`).join(' | ') + ' |');
  }
  lines.push('', '## Screen-only user per viewport (actions; ✗ = target not reached)', '', '| task | ' + viewports.map((v) => v.name).join(' | ') + ' |', '|---|' + viewports.map(() => '---').join('|') + '|');
  for (const t of tasks) lines.push(`| ${t.id} | ` + viewports.map((vp) => themes.map((th) => {
    const r = runs.find((x) => x.task === t.id && x.viewport === vp.name && x.theme === th);
    return r ? (r.done ? String(r.metrics['screen_only.actions']) : '✗' + r.metrics['screen_only.actions']) : '–';
  }).join(' / ')).join(' | ') + ' |');
  lines.push('', 'Cells are light / dark; – = not run. Gate 5 counts include the known loss link-in-text-block (UXF-002); held = gate 2 sidebar at page end (UXF-003). Search is absent on every page (UXF-001, held).', '', '## Worst moments', '');
  if (!worstShots.length) lines.push('None: no layout violation in any viewport.');
  for (const s of worstShots) lines.push(`- gate ${s.v.gate}, ${s.count}×: \`${s.v.what}\` ${s.v.detail} — first at \`${s.v.route}\` ${s.v.viewport} ${s.v.theme} (${s.v.state}): [shots/${s.file}](shots/${s.file})`);
  lines.push('', `Contact sheets: [light](shots/contact-sheet-light.png), [dark](shots/contact-sheet-dark.png).`, '');
  lines.push('## Not run this round', '', '- L4 replayed real sessions and L5 discoverability probe: no paid API and no key in this round.', '- L6 recorded live rounds and the questionnaire: no person took part; no questionnaire scores.', '- L0 engine scenarios and gate 6: the site has no guidance cue.', '');
  writeFileSync(path.join(dir, 'report.md'), lines.join('\n') + '\n');

  // trend.md: one row per round.
  const trendFile = path.join(ux, 'rounds', 'trend.md');
  const head = ['# Trend', '', 'One row per round, from each round\'s `round.json`. Gate columns sum the gating viewports (1366x768, 1280x720) over both themes.', '', '| round | gate 1 | gate 2 | gate 3 | gate 4 | gate 5 | tasks done (all viewports) | actions, sum of task worst |', '|---|---|---|---|---|---|---|---|'];
  const sum = (g: 'gate1' | 'gate2' | 'gate3' | 'gate4' | 'gate5') => gatingViewports.reduce((s, vp) => s + themes.reduce((t, th) => t + (gates[vp.name][th][g] as number), 0), 0);
  const row = `| ${round} | ${sum('gate1')} | ${sum('gate2')} | ${sum('gate3')} | ${sum('gate4')} | ${sum('gate5')} | ${runs.filter((r) => r.done).length}/${runs.length} | ${Object.values(worst).reduce((s, w) => s + (w['screen_only.actions'] ?? 0), 0)} |`;
  const old = existsSync(trendFile) ? readFileSync(trendFile, 'utf8').split('\n').filter((l) => /^\| R-\d+ /.test(l) && !l.startsWith(`| ${round} `)) : [];
  writeFileSync(trendFile, [...head, ...[...old, row].sort()].join('\n') + '\n');
});

// Worst moments: the first instance of each of the eight largest violation groups, one test each.
for (let i = 0; i < 8; i++)
  test(`worst moment ${i + 1}`, async ({page}) => {
    const g = violationGroups(read('layout'))[i];
    test.skip(!g, 'fewer violation groups');
    const {file, v} = worstOf(g, i);
    const vp = viewports.find((x) => x.name === v.viewport)!;
    mkdirSync(shots, {recursive: true});
    await page.setViewportSize({width: vp.width, height: vp.height});
    await chooseTheme(page, v.theme);
    await visit(page, site.baseUrl, v.route);
    await page.evaluate((s) => window.scrollTo(0, s === 'top' ? 0 : document.documentElement.scrollHeight), v.state);
    await settle(page);
    await page.waitForTimeout(750);
    await page.screenshot({path: path.join(shots, file)});
  });

// Contact sheet per theme from the key-moment frames.
for (const theme of themes)
  test(`contact sheet ${theme}`, async ({page}) => {
    mkdirSync(shots, {recursive: true});
    const list = existsSync(frames) ? readdirSync(frames).filter((f) => f.startsWith(theme + '--')).sort() : [];
    const cells = list
      .map((f) => `<figure><img src="data:image/png;base64,${readFileSync(path.join(frames, f)).toString('base64')}"><figcaption>${f.replace(/^\w+--/, '').replace(/\.png$/, '').replace(/_/g, '/')}</figcaption></figure>`)
      .join('');
    await page.setViewportSize({width: 1920, height: 1080});
    await page.setContent(`<style>body{margin:0;padding:12px;background:${theme === 'dark' ? '#111' : '#eee'};font:12px monospace;color:${theme === 'dark' ? '#ddd' : '#222'}}main{display:grid;grid-template-columns:repeat(4,1fr);gap:10px}figure{margin:0}img{width:100%;border:1px solid #888}</style><h1>${round} contact sheet, ${theme}, ${gatingViewports[0].name}, 0.75 s after each moment</h1><main>${cells}</main>`);
    await page.waitForLoadState('load');
    await page.screenshot({path: path.join(shots, `contact-sheet-${theme}.png`), fullPage: true});
  });
