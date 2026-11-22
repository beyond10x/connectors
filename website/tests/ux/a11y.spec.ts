// L2 gate 5 (skill § 4): axe-core reports no serious or critical violation in either colour mode,
// on one page of each template, and every task can be done by keyboard alone. The fast loop runs
// axe at the first gating viewport: every rule of the tags in the first colour mode, and in the
// others the rules a colour mode can change (a mode changes colours only); the round runs every
// rule at every viewport in both modes.
import {axeTags, colourRules, gatingViewports, knownAxe, knownAxeProbe, site, tasks, fastAxePages, themes} from '../../ux/adapter';
import {axe, serious} from './core/axe';
import {record} from './core/contract';
import {chooseTheme, currentTheme, expect, test, visit} from './core/fixture';
import {keyboardOnly} from './core/keyboard';

const vp = gatingViewports[0];

for (const [n, theme] of themes.entries()) {
  for (const route of fastAxePages) {
    test(`axe ${vp.name} ${theme} ${route}`, async ({page, offsite}) => {
      await page.setViewportSize({width: vp.width, height: vp.height});
      await chooseTheme(page, theme);
      await visit(page, site.baseUrl, route);
      expect(await currentTheme(page)).toBe(theme);
      const all = serious(await axe(page, axeTags, n === 0 ? undefined : colourRules));
      const hits = all.filter((h) => !knownAxe[h.id]);
      record('axe', `${vp.name}@${theme}@${route}`, {route, viewport: vp.name, theme, hits: all});
      expect(offsite).toEqual([]);
      expect(hits, hits.map((h) => `${h.id} (${h.impact}): ${h.nodes.map((n) => n.target).join(' | ')}`).join('\n')).toEqual([]);
    });
  }
}

// UXF-004: the site's own table rule made a wrapped table its own scroller, a box the keyboard
// cannot reach; Docs System's wrapper scrolls and takes focus instead.
for (const v of gatingViewports) {
  test(`UXF-004: wide tables scroll in a box the keyboard reaches, ${v.name}`, async ({page}) => {
    await page.setViewportSize({width: v.width, height: v.height});
    await chooseTheme(page, 'light');
    await visit(page, site.baseUrl, '/docs/reference/contracts/cli');
    const hits = await axe(page, axeTags, ['scrollable-region-focusable']);
    expect(hits.map((h) => `${h.id}: ${h.nodes.map((n) => n.target).join(' | ')}`)).toEqual([]);
    const wraps = await page.evaluate(() => [...document.querySelectorAll('.b10x-table-wrap')].filter((w) => w.scrollWidth > w.clientWidth + 1).map((w) => (w as HTMLElement).tabIndex));
    expect(wraps.length, 'a table wider than the column at this size').toBeGreaterThan(0);
    expect(wraps.every((t) => t === 0), 'every overflowing wrapper takes focus').toBe(true);
  });
}

test('known losses of the shared theme still show; drop each from knownAxe once it is gone', async ({page}) => {
  await page.setViewportSize({width: vp.width, height: vp.height});
  await chooseTheme(page, 'light');
  await visit(page, site.baseUrl, knownAxeProbe);
  const ids = serious(await axe(page, axeTags)).map((h) => h.id);
  for (const [rule, finding] of Object.entries(knownAxe)) expect(ids, `${rule} (${finding}) no longer shows`).toContain(rule);
});

// Keyboard alone: the anchor cards here; the round runs every task (a long reference page costs
// hundreds of Tab presses, which the 120 s budget cannot hold under load).
for (const task of tasks.filter((t) => t.card === 'anchor')) {
  test(`keyboard alone: ${task.id}`, async ({page}) => {
    await page.setViewportSize({width: vp.width, height: vp.height});
    await chooseTheme(page, 'light');
    const run = await keyboardOnly(page, site.baseUrl, task);
    record('keyboard', task.id, run);
    expect(run.done, `${task.id} by keyboard: ${run.path.join(' → ')} after ${run.presses} presses`).toBe(true);
    expect(run.focusVisible, 'focus ring visible on every followed control').toBe(true);
  });
}

test('break: gate 5 sees an image without a text alternative and an unnamed button', async ({page}) => {
  await page.setViewportSize({width: vp.width, height: vp.height});
  await chooseTheme(page, 'light');
  await visit(page, site.baseUrl, '/docs');
  await page.evaluate(() => {
    const img = document.createElement('img');
    img.src = 'data:image/gif;base64,R0lGODlhAQABAAAAACw=';
    img.width = 20;
    img.height = 20;
    document.body.append(img, document.createElement('button'));
  });
  const ids = serious(await axe(page, axeTags)).map((h) => h.id);
  expect(ids).toContain('image-alt');
  expect(ids).toContain('button-name');
});
