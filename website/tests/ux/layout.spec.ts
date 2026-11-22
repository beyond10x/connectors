// L2 layout contract, gates 1 and 2 (skill § 4), at the gating viewports in both colour modes, on
// one page per template and every task's page (the round sweeps the whole sitemap): see sweep.ts.
import {alwaysVisible, fastPages, gatingViewports, knownHiddenProbe, site} from '../../ux/adapter';
import {record, type Violation} from './core/contract';
import {chooseTheme, expect, settle, test, visit} from './core/fixture';
import {overflowProbe, visibilityProbe} from './core/probes';
import {sweep} from './sweep';

const all = fastPages();
const chunk = 4;
const toggle = alwaysVisible.find((a) => a.name === 'theme toggle')!.selector;
const sidebar = alwaysVisible.find((a) => a.name === 'sidebar')!.selector;

for (const vp of gatingViewports) {
  for (let i = 0; i < all.length; i += chunk) {
    const routes = all.slice(i, i + chunk);
    test(`layout ${vp.name} ${routes.join(' ')}`, async ({page, offsite}) => {
      await page.setViewportSize({width: vp.width, height: vp.height});
      await chooseTheme(page, 'light');
      const violations: Violation[] = [];
      const held: Violation[] = [];
      for (const route of routes) {
        const r = await sweep(page, route, vp);
        violations.push(...r.violations);
        held.push(...r.held);
      }
      record('layout', `${vp.name}@${i}`, {routes, violations, held});
      expect(offsite, 'requests off the local build').toEqual([]);
      expect(violations).toEqual([]);
    });
  }
}

test('known loss UXF-003 still shows: the sidebar leaves the viewport at the end of a page; drop it from knownHidden once it is gone', async ({page}) => {
  await page.setViewportSize(gatingViewports[0]);
  await visit(page, site.baseUrl, knownHiddenProbe);
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  await settle(page);
  const v = await page.evaluate(visibilityProbe, sidebar);
  expect(v.inViewport && !v.clippedBy && v.coveredAt.length === 0).toBe(false);
});

// Break tests: each check can fail.
test('break: gate 1 sees a box running past the viewport and a clipped label', async ({page}) => {
  await page.setViewportSize(gatingViewports[0]);
  await visit(page, site.baseUrl, '/docs');
  await page.evaluate(() => {
    const wide = document.createElement('p');
    wide.style.width = '3000px';
    wide.textContent = 'wide';
    document.body.append(wide);
    const tight = document.createElement('p');
    tight.style.cssText = 'width:40px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis';
    tight.textContent = 'a label that does not fit';
    document.body.append(tight);
  });
  const kinds = (await page.evaluate(overflowProbe)).map((o) => o.kind);
  expect(kinds.some((k) => k === 'page-scroll-x' || k === 'past-viewport' || k === 'runs-past')).toBe(true);
  expect(kinds).toContain('clipped');
});

test('break: gate 2 sees a covered theme toggle and a sidebar pushed out of view', async ({page}) => {
  await page.setViewportSize(gatingViewports[0]);
  await visit(page, site.baseUrl, '/docs');
  await page.evaluate(() => {
    const cover = document.createElement('div');
    cover.id = 'ux-cover';
    cover.style.cssText = 'position:fixed;inset:0;z-index:10000;background:rgb(0 0 0 / 1%)';
    document.body.append(cover);
  });
  expect((await page.evaluate(visibilityProbe, toggle)).coveredAt).toContain('centre');
  await page.evaluate(() => document.getElementById('ux-cover')!.remove());
  await page.addStyleTag({content: 'aside.theme-doc-sidebar-container nav.menu{transform:translateY(-200px)}'});
  const s = await page.evaluate(visibilityProbe, sidebar);
  expect(s.inViewport && !s.clippedBy && s.coveredAt.length === 0).toBe(false);
});
