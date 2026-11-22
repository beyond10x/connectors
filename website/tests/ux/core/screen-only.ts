// The screen-only user (skill § 7) in docs-site form. Deterministic code, no model: at each state
// it looks at the controls fully in view and not covered, picks the one whose accessible name
// matches the task's words best, and clicks its centre. With nothing to pick it scrolls one screen
// (a look); at the end of a page with nothing to pick it goes back (a steer). It is done when the
// task's target is on screen at the task's page.
import type {Page} from 'playwright/test';
import {controlsProbe, goalProbe, readingLoadProbe, type Control} from './probes';
import {settle} from './fixture';

export type BotTask = {id: string; start: string; goal: {path: string; hash?: string; text: RegExp}; words: RegExp[]};

export type Step = {
  state: number;
  url: string;
  action: 'click' | 'scroll' | 'back';
  control?: string;
  region?: Control['region'];
  glance?: 1 | 2 | 3;
  words: number;
};

export type Run = {
  task: string;
  done: boolean;
  endPath: string;
  steps: Step[];
  metrics: {
    'screen_only.actions': number;
    'screen_only.looks': number;
    'screen_only.steers': number;
    'screen_only.time_to_act_steps.max': number;
    'glance_cost.median': number;
    'glance_cost.p95': number;
    'reading_load.max': number;
    'reading_load.median': number;
    'layout_shift.score': number;
    completion_miss: number;
    false_complete: number;
  };
};

// Patience: a task not done in 30 actions fails gate 4; the cost below that is the ratchet's.
const MAX_ACTIONS = 30;

const quantile = (xs: number[], q: number) => {
  if (!xs.length) return 0;
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.ceil(q * s.length) - 1)];
};

async function layoutShift(page: Page): Promise<number> {
  return page.evaluate(() => ((window as unknown as {__uxShift?: number}).__uxShift ?? 0));
}

export async function installShiftObserver(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const w = window as unknown as {__uxShift: number};
    w.__uxShift = Number(sessionStorage.getItem('__uxShift') ?? 0);
    new PerformanceObserver((list) => {
      for (const e of list.getEntries() as unknown as {value: number; hadRecentInput: boolean}[]) if (!e.hadRecentInput) w.__uxShift += e.value;
      sessionStorage.setItem('__uxShift', String(w.__uxShift));
    }).observe({type: 'layout-shift', buffered: true});
  });
}

export async function goalOnScreen(page: Page, task: BotTask, base: string) {
  const at = new URL(page.url());
  const path = at.pathname.replace(/\/$/, '') || '/';
  const want = (base + task.goal.path).replace(/\/$/, '');
  // The page decides; the fragment only matters for a direct visit (completion_miss).
  const onPage = path === want;
  const g = await page.evaluate(goalProbe, {source: task.goal.text.source, flags: task.goal.text.flags});
  return {onPage, path, ...g};
}

// `scrollers`: navigation boxes that scroll on their own (a docs sidebar), from the adapter.
// `openers`: names of controls that open hidden navigation (a mobile menu), tried once per state
// before scrolling.
export async function play(page: Page, base: string, task: BotTask, scrollers: string[], onDone?: () => Promise<void>, openers: RegExp[] = []): Promise<Run> {
  await installShiftObserver(page);
  await page.goto(base + task.start, {waitUntil: 'load'});
  await settle(page);
  const steps: Step[] = [];
  const visited = new Set<string>();
  const tried = new Set<string>();
  // The words are ordered best first; once a word led somewhere, only better words are followed.
  const ranks: number[] = [task.words.length];
  let state = 0;
  let looksThisState = 0;
  let navLooks = 0;
  const lookRuns: number[] = [];
  let falseComplete = 0;
  let coveredSeen = 0;
  let done = false;
  const key = (u: string) => u.replace(/#.*$/, '').replace(/\/$/, '');
  while (steps.length < MAX_ACTIONS) {
    const g = await goalOnScreen(page, task, base);
    if (g.found && g.visible && !g.covered) {
      if (!g.onPage) falseComplete = 1;
      done = true;
      if (onDone) await onDone();
      break;
    }
    if (g.onPage && g.found && g.visible && g.covered) coveredSeen = 1;
    const here = page.url();
    visited.add(key(here));
    const words = await page.evaluate(readingLoadProbe);
    const controls = await page.evaluate(controlsProbe);
    const origin = new URL(here).origin;
    const pick = choose(controls, task, here, visited, tried, origin, base, ranks.at(-1)!);
    if (pick) {
      tried.add(key(here) + '|' + pick.name);
      const glance: 1 | 2 | 3 = looksThisState > 0 ? 3 : pick.region === 'main' ? 1 : 2;
      steps.push({state, url: here, action: 'click', control: pick.name, region: pick.region, glance, words});
      lookRuns.push(looksThisState);
      ranks.push(task.words.findIndex((re) => re.test(pick.name)));
      await page.mouse.click(pick.x, pick.y);
      if (pick.href && key(pick.href) !== key(here)) await page.waitForURL((u) => key(u.href) === key(pick.href!), {timeout: 10_000}).catch(() => {});
      await settle(page);
      state++;
      looksThisState = 0;
      navLooks = 0;
      continue;
    }
    // Nothing to follow in view. Arrived at a place on a page (a link with a fragment), or at the
    // best word already: read on down the page. Otherwise scan the navigation first: scroll a
    // navigation box that holds more, one box-height at a time, at most four times per state.
    const opener = controls.find((c) => openers.some((re) => re.test(c.name)) && !c.expanded && !tried.has(key(here) + '|' + c.name));
    if (opener) {
      tried.add(key(here) + '|' + opener.name);
      steps.push({state, url: here, action: 'click', control: opener.name, region: opener.region, glance: looksThisState > 0 ? 3 : 2, words});
      await page.mouse.click(opener.x, opener.y);
      await settle(page);
      continue;
    }
    const reading = new URL(here).hash !== '' || ranks.at(-1) === 0;
    const scrolled = !reading && navLooks < 4 && (await page.evaluate((sel) => {
      for (const s of sel) {
        const el = document.querySelector(s);
        if (!el || el.getBoundingClientRect().width === 0) continue;
        if (el.scrollTop + el.clientHeight >= el.scrollHeight - 2) continue;
        el.scrollBy(0, Math.round(el.clientHeight * 0.8));
        return true;
      }
      return false;
    }, scrollers));
    if (scrolled) {
      steps.push({state, url: here, action: 'scroll', words});
      looksThisState++;
      navLooks++;
      await settle(page);
      continue;
    }
    const atEnd = await page.evaluate(() => window.scrollY + window.innerHeight >= document.documentElement.scrollHeight - 2);
    if (!atEnd) {
      steps.push({state, url: here, action: 'scroll', words});
      looksThisState++;
      // A wheel turn over the content, the way a reader scrolls: a key press would scroll whatever
      // box holds focus (the table of contents after a click in it).
      const vp = page.viewportSize()!;
      await page.mouse.move(Math.round(vp.width * 0.55), Math.round(vp.height * 0.6));
      await page.mouse.wheel(0, Math.round(vp.height * 0.85));
      await settle(page);
      continue;
    }
    steps.push({state, url: here, action: 'back', words});
    lookRuns.push(looksThisState);
    if (ranks.length > 1) ranks.pop();
    await page.goBack({waitUntil: 'load'}).catch(() => {});
    await settle(page);
    state++;
    looksThisState = 0;
    navLooks = 0;
  }
  // The completion state must also be unmissable on a direct visit: open the target the way a
  // shared link opens it and check that it is on screen and not under the navbar.
  const target = base + task.goal.path + (task.goal.hash ? '#' + task.goal.hash : '');
  const endPath = new URL(page.url()).pathname;
  await page.goto(target, {waitUntil: 'load'});
  await settle(page);
  const direct = await goalOnScreen(page, task, base);
  const directMiss = task.goal.hash && !(direct.found && direct.visible && !direct.covered) ? 1 : 0;
  const completionMiss = Math.max(directMiss, coveredSeen);
  const glances = steps.filter((s) => s.glance).map((s) => s.glance!);
  const loads = steps.map((s) => s.words);
  return {
    task: task.id,
    done,
    endPath,
    steps,
    metrics: {
      'screen_only.actions': steps.length,
      'screen_only.looks': steps.filter((s) => s.action === 'scroll').length,
      'screen_only.steers': steps.filter((s) => s.action === 'back').length,
      'screen_only.time_to_act_steps.max': Math.max(0, ...lookRuns),
      'glance_cost.median': quantile(glances, 0.5),
      'glance_cost.p95': quantile(glances, 0.95),
      'reading_load.max': Math.max(0, ...loads),
      'reading_load.median': quantile(loads, 0.5),
      'layout_shift.score': Math.round((await layoutShift(page)) * 1000) / 1000,
      completion_miss: completionMiss,
      false_complete: falseComplete,
    },
  };
}

const before = (a: number[], b: number[]) => {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] < b[i];
  return false;
};

const regionRank: Record<Control['region'], number> = {main: 0, toc: 1, sidebar: 2, navbar: 3, footer: 4, other: 5};

function choose(controls: Control[], task: BotTask, here: string, visited: Set<string>, tried: Set<string>, origin: string, base: string, below: number): Control | null {
  const key = (u: string) => u.replace(/#.*$/, '').replace(/\/$/, '');
  const goalUrl = origin + base + task.goal.path;
  let best: {c: Control; score: number[]} | null = null;
  for (const c of controls) {
    const w = task.words.findIndex((re) => re.test(c.name));
    if (w < 0 || w >= below || c.expanded || tried.has(key(here) + '|' + c.name)) continue;
    if (c.href) {
      if (!c.href.startsWith(origin + base)) continue;
      const sameDoc = key(c.href) === key(here);
      // A link to the page in view moves only when it names another place on it; a page already
      // seen is not visited twice, except the goal page itself.
      if (sameDoc && (!c.href.includes('#') || c.href === here) && !c.href.endsWith('#')) continue;
      if (!sameDoc && visited.has(key(c.href)) && key(c.href) !== key(goalUrl)) continue;
    }
    const score = [w, regionRank[c.region], Math.round(c.y), Math.round(c.x)];
    if (!best || before(score, best.score)) best = {c, score};
  }
  return best?.c ?? null;
}
