// Gate 5, keyboard alone: from the task's start page, Tab moves focus and Enter follows the first
// control in tab order whose accessible name matches a task word better than the last one followed;
// on the task's page, Page Down until the target is on screen. Every key press counts. The tab
// order is read from the page so that the presses run in one batch; the focused control is then
// checked against it.
import type {Page} from 'playwright/test';
import {settle} from './fixture';
import {goalOnScreen, type BotTask} from './screen-only';

const MAX_PRESSES = 1500;

export type KeyRun = {task: string; done: boolean; presses: number; path: string[]; focusVisible: boolean; drift: number; stuck?: string[]};

type Next = {tabs: number; name: string; href: string | null; word: number} | null;

function nextInTabOrder(arg: {words: [string, string][]; below: number; skip: string[]; origin: string; here: string}): Next {
  const res = arg.words.map(([s, f]) => new RegExp(s, f));
  const key = (u: string) => u.replace(/#.*$/, '').replace(/\/$/, '');
  const shown = (el: HTMLElement) => {
    if (el.closest('[inert], [hidden]')) return false;
    const r = el.getBoundingClientRect();
    if (r.width === 0 && r.height === 0) return false;
    const cs = getComputedStyle(el);
    return cs.visibility !== 'hidden';
  };
  const all = [...document.querySelectorAll<HTMLElement>('a[href], button, input, select, textarea, summary, [tabindex]')].filter(
    (el) => el.tabIndex >= 0 && !(el as HTMLButtonElement).disabled && shown(el),
  );
  // Focus may sit on an element out of the tab order (a route change focuses the content): Tab
  // then moves to the next tabbable element after it in document order.
  const active = document.activeElement as HTMLElement | null;
  const inList = active ? all.indexOf(active) : -1;
  const before = active && active !== document.body ? all.filter((el) => el.compareDocumentPosition(active) & Node.DOCUMENT_POSITION_FOLLOWING).length : 0;
  const at = inList >= 0 ? inList : before - 1;
  // Forward first, then back (Shift+Tab): the first match in either direction.
  const order: number[] = [];
  for (let i = at + 1; i < all.length; i++) order.push(i);
  for (let i = inList >= 0 ? at - 1 : at; i >= 0; i--) order.push(i);
  for (const i of order) {
    const el = all[i];
    const name = (el.getAttribute('aria-label') || (el.innerText || '').replace(/\s+/g, ' ').trim() || el.getAttribute('title') || '').trim();
    const word = res.findIndex((re) => re.test(name));
    if (word < 0 || word >= arg.below || el.getAttribute('aria-expanded') === 'true') continue;
    const href = el instanceof HTMLAnchorElement ? el.href : null;
    if (arg.skip.includes(key(arg.here) + '|' + name)) continue;
    if (href && (!href.startsWith(arg.origin) || href === arg.here || (key(href) === key(arg.here) && !href.includes('#')))) continue;
    if (href && key(href) !== key(arg.here) && arg.skip.includes('visited|' + key(href))) continue;
    return {tabs: i > at ? i - at : -(at - i + (inList >= 0 ? 0 : 1)), name, href, word};
  }
  return null;
}

export async function keyboardOnly(page: Page, base: string, task: BotTask): Promise<KeyRun> {
  await page.goto(base + task.start, {waitUntil: 'load'});
  await settle(page);
  let presses = 0;
  let drift = 0;
  const path: string[] = [];
  const key = (u: string) => u.replace(/#.*$/, '').replace(/\/$/, '');
  const skip: string[] = ['visited|' + key(page.url())];
  const ranks = [task.words.length];
  let focusVisible = true;
  const misses = new Map<string, number>();
  let stuck: string[] | undefined;
  const words = task.words.map((re) => [re.source, re.flags] as [string, string]);
  while (presses < MAX_PRESSES) {
    const g = await goalOnScreen(page, task, base);
    if (g.onPage && g.found && g.visible && !g.covered) return {task: task.id, done: true, presses, path, focusVisible, drift};
    if (g.onPage && g.found) {
      const atEnd = await page.evaluate(() => window.scrollY + window.innerHeight >= document.documentElement.scrollHeight - 2);
      if (atEnd) break;
      await page.keyboard.press('PageDown');
      presses++;
      await settle(page);
      continue;
    }
    const here = page.url();
    const next = await page.evaluate(nextInTabOrder, {words, below: ranks.at(-1)!, skip, origin: new URL(here).origin + base, here});
    if (!next) {
      // Say what was there: every focusable control whose name matches a task word.
      stuck = await page.evaluate((ws) => {
        const res = ws.map(([s, f]) => new RegExp(s, f));
        return [...document.querySelectorAll<HTMLElement>('a[href], button')]
          .map((el) => `${(el.getAttribute('aria-label') || (el.innerText || '').replace(/\s+/g, ' ').trim()).slice(0, 40)} tab=${el.tabIndex} h=${Math.round(el.getBoundingClientRect().height)} exp=${el.getAttribute('aria-expanded')}`)
          .filter((s) => res.some((re) => re.test(s.split(' tab=')[0])))
          .slice(0, 12);
      }, words);
      stuck.unshift(`at ${page.url()} active=${await page.evaluate(() => document.activeElement?.tagName + ' ' + (document.activeElement?.textContent ?? '').slice(0, 30))}`);
      break;
    }
    const keyName = next.tabs > 0 ? 'Tab' : 'Shift+Tab';
    for (let i = 0; i < Math.abs(next.tabs); i++) await page.keyboard.press(keyName);
    presses += Math.abs(next.tabs);
    const focused = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el) return null;
      const cs = getComputedStyle(el);
      return {name: (el.getAttribute('aria-label') || (el.innerText || '').replace(/\s+/g, ' ').trim() || el.getAttribute('title') || '').trim(), ring: (cs.outlineStyle !== 'none' && parseFloat(cs.outlineWidth) > 0) || cs.boxShadow !== 'none'};
    });
    // The browser's tab order differs from the page's reading of it: count it, then follow what
    // has focus only if it is the control the reading named.
    if (!focused || focused.name !== next.name) {
      drift++;
      // Try again from where focus landed; give up on that control after the third miss.
      const id = key(here) + '|' + next.name;
      misses.set(id, (misses.get(id) ?? 0) + 1);
      if (misses.get(id)! >= 3) skip.push(id);
      continue;
    }
    if (!focused.ring) focusVisible = false;
    skip.push(key(here) + '|' + next.name);
    await page.keyboard.press('Enter');
    presses++;
    path.push(next.name);
    ranks.push(next.word);
    if (next.href && key(next.href) !== key(here)) await page.waitForURL((u) => key(u.href) === key(next.href!), {timeout: 10_000}).catch(() => {});
    await settle(page);
    skip.push('visited|' + key(page.url()));
  }
  return {task: task.id, done: false, presses, path, focusVisible, drift, stuck};
}
