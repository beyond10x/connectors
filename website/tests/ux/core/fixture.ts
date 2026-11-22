// The hermetic page of every ux test: only the local build answers, every other request is
// aborted and counted, and the colour mode is chosen before the first script runs.
import {test as base, expect, type Page} from 'playwright/test';

export type Theme = string;

export const test = base.extend<{origin: string; offsite: string[]}>({
  origin: async ({baseURL}, use) => use(new URL(baseURL!).origin),
  offsite: [
    async ({page, origin}, use) => {
      const blocked: string[] = [];
      await page.route('**/*', (route) => {
        const url = route.request().url();
        if (url.startsWith(origin) || url.startsWith('data:') || url.startsWith('blob:')) return route.continue();
        blocked.push(url);
        return route.abort('blockedbyclient');
      });
      await use(blocked);
    },
    {auto: true},
  ],
});

export {expect};

export async function chooseTheme(page: Page, theme: Theme): Promise<void> {
  await page.emulateMedia({colorScheme: theme === 'dark' ? 'dark' : 'light'});
  // sessionStorage 'ux-theme' overrides the mode for the rest of the tab (see forceTheme).
  await page.addInitScript((mode) => {
    try {
      localStorage.setItem('theme', sessionStorage.getItem('ux-theme') ?? mode);
    } catch {}
  }, theme);
}

export async function forceTheme(page: Page, theme: Theme | null): Promise<void> {
  await page.evaluate((mode) => (mode ? sessionStorage.setItem('ux-theme', mode) : sessionStorage.removeItem('ux-theme')), theme);
}

export async function settle(page: Page): Promise<void> {
  await page.waitForLoadState('load');
  // Docusaurus marks the document once React has hydrated it; before that, controls do nothing.
  await page.waitForSelector('html[data-has-hydrated="true"]', {state: 'attached', timeout: 15_000}).catch(() => {});
  await page.evaluate(async () => {
    await document.fonts.ready;
    let last = -1;
    for (let i = 0; i < 30; i++) {
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      if (window.scrollY === last) break;
      last = window.scrollY;
    }
  });
}

export async function visit(page: Page, base: string, route: string): Promise<void> {
  await page.goto(base + route, {waitUntil: 'load'});
  await settle(page);
}

export async function currentTheme(page: Page): Promise<string | null> {
  return page.locator('html').getAttribute('data-theme');
}

// The theme toggle cycles light, dark and system; press it until the mode is reached, the way a
// reader does.
export async function toggleTo(page: Page, theme: Theme, toggle: string): Promise<number> {
  let presses = 0;
  while ((await currentTheme(page)) !== theme && presses < 3) {
    await page.locator(toggle).first().click();
    presses++;
    await settle(page);
  }
  expect(await currentTheme(page)).toBe(theme);
  return presses;
}
