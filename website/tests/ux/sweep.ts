// One page through gates 1 and 2: loaded once in light mode, checked at its top and its end, then
// the theme toggle is pressed to dark (a state change) and both are checked again. Shared by the
// fast loop (layout.spec.ts) and the round (measure.spec.ts).
import type {Page} from 'playwright/test';
import {alwaysVisible, desktopWidth, isDocsPage, knownAbsent, knownHidden, site, type Viewport} from '../../ux/adapter';
import {gate1, gate2, type Violation} from './core/contract';
import {forceTheme, settle, toggleTo, visit} from './core/fixture';

const toggle = alwaysVisible.find((a) => a.name === 'theme toggle')!.selector;

export async function sweep(page: Page, route: string, vp: Viewport): Promise<{violations: Violation[]; held: Violation[]}> {
  const violations: Violation[] = [];
  const held: Violation[] = [];
  await visit(page, site.baseUrl, route);
  const desktop = vp.width >= desktopWidth;
  for (const theme of ['light', 'dark']) {
    if (theme === 'dark') {
      if (desktop) await toggleTo(page, 'dark', toggle);
      else {
        // Below the desktop width the toggle sits in the navigation drawer; store the mode and
        // reload instead.
        await forceTheme(page, 'dark');
        await page.emulateMedia({colorScheme: 'dark'});
        await page.reload({waitUntil: 'load'});
        await settle(page);
        await forceTheme(page, null);
        await page.emulateMedia({colorScheme: 'light'});
      }
    }
    for (const state of theme === 'light' ? ['top', 'end'] : ['end', 'top']) {
      await page.evaluate((s) => window.scrollTo(0, s === 'top' ? 0 : document.documentElement.scrollHeight), state);
      await settle(page);
      const at = {route, viewport: vp.name, theme, state};
      if (state === 'top') violations.push(...(await gate1(page, at)));
      for (const v of (await gate2(page, alwaysVisible, at, knownAbsent, isDocsPage(route), desktop)).violations)
        (knownHidden[`${v.what}@${v.state}`] ? held : violations).push(v);
    }
  }
  return {violations, held};
}
