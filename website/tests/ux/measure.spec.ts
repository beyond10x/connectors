// The round command's Measure step (skill § 3.2), deterministic and keyless. Gates 1 and 2 over the
// whole sitemap at the gating viewports and over the fast loop's pages at the measured-only ones;
// axe-core on every template page, every task by the screen-only user, at every viewport in both
// colour modes; the keyboard-only runs; the key-moment frames. Each test writes its part under
// .cache/ux/results/measure/; round-report.spec.ts then writes ux/rounds/<UX_ROUND>/.
import {mkdirSync} from 'node:fs';
import path from 'node:path';
import {axeTags, fastPages, gatingViewports, measuredViewports, navOpeners, navScrollers, pages, site, tasks, templatePages, themes} from '../../ux/adapter';
import {axe} from './core/axe';
import {record, type Violation} from './core/contract';
import {chooseTheme, test, visit} from './core/fixture';
import {keyboardOnly} from './core/keyboard';
import {play} from './core/screen-only';
import {sweep} from './sweep';

const round = process.env.UX_ROUND ?? 'R-00';
const frames = path.join(__dirname, '..', '..', '.cache', 'ux', 'frames', round);
const viewports = [...gatingViewports, ...measuredViewports];
const chunk = 5;

// The whole sitemap at the first gating viewport; the fast loop's pages at the others. axe-core and
// the screen-only user in both colour modes at the first gating viewport and in the first mode at
// the others (axe on the template pages at the gating viewports, on four pages at the measured-only
// ones). Sized for a loaded shared host; a round runs in two parts (README).
const quickAxe = ['/', '/docs', '/docs/getting-started', '/docs/connectors/gitlab'];

for (const vp of viewports) {
  const gated = gatingViewports.includes(vp);
  const list = vp === gatingViewports[0] ? pages() : fastPages();
  for (let i = 0; i < list.length; i += chunk) {
    const routes = list.slice(i, i + chunk);
    test(`layout ${vp.name} ${i}`, async ({page}) => {
      await page.setViewportSize({width: vp.width, height: vp.height});
      await chooseTheme(page, 'light');
      const violations: Violation[] = [];
      const held: Violation[] = [];
      for (const route of routes) {
        const r = await sweep(page, route, vp);
        violations.push(...r.violations);
        held.push(...r.held);
      }
      record('measure/layout', `${vp.name}@${i}`, {viewport: vp.name, routes: routes.length, violations, held});
    });
  }
  for (const theme of vp === gatingViewports[0] ? themes : themes.slice(0, 1)) {
    for (const route of gated ? templatePages : quickAxe) {
      test(`axe ${vp.name} ${theme} ${route}`, async ({page}) => {
        await page.setViewportSize({width: vp.width, height: vp.height});
        await chooseTheme(page, theme);
        await visit(page, site.baseUrl, route);
        record('measure/axe', `${vp.name}@${theme}@${route}`, {viewport: vp.name, theme, pages: [{route, hits: await axe(page, axeTags)}]});
      });
    }
    for (const task of tasks) {
      test(`screen-only ${task.id} ${vp.name} ${theme}`, async ({page}) => {
        await page.setViewportSize({width: vp.width, height: vp.height});
        await chooseTheme(page, theme);
        const run = await play(page, site.baseUrl, task, navScrollers, undefined, navOpeners);
        record('measure/screen-only', `${task.id}@${vp.name}@${theme}`, {viewport: vp.name, theme, ...run});
      });
    }
  }
}

for (const task of tasks) {
  test(`keyboard ${task.id}`, async ({page}) => {
    await page.setViewportSize(gatingViewports[0]);
    await chooseTheme(page, 'light');
    record('measure/keyboard', task.id, await keyboardOnly(page, site.baseUrl, task));
  });
}

// Key moments for the contact sheet, at the first gating viewport, each 0.75 s after its moment:
// every template page as it opens, and every task's end state as the screen-only user reaches it.
for (const theme of themes) {
  for (const route of templatePages) {
    test(`frames ${theme} page ${route}`, async ({page}) => {
      mkdirSync(frames, {recursive: true});
      await page.setViewportSize(gatingViewports[0]);
      await chooseTheme(page, theme);
      await visit(page, site.baseUrl, route);
      await page.waitForTimeout(750);
      await page.screenshot({path: path.join(frames, `${theme}--page${route.replace(/\//g, '_')}.png`)});
    });
  }
  for (const task of tasks) {
    test(`frames ${theme} task ${task.id}`, async ({page}) => {
      mkdirSync(frames, {recursive: true});
      await page.setViewportSize(gatingViewports[0]);
      await chooseTheme(page, theme);
      await play(page, site.baseUrl, task, navScrollers, async () => {
        await page.waitForTimeout(750);
        await page.screenshot({path: path.join(frames, `${theme}--task_${task.id}.png`)});
      });
    });
  }
}
