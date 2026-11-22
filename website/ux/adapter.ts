// The site's adapter for the ui-iteration-loop (skill § 1): everything the generic core under
// tests/ux/core/ needs to know about this site. The core imports nothing from src/.
import {readFileSync} from 'node:fs';
import path from 'node:path';

export type Viewport = {name: string; width: number; height: number};
export type Theme = 'light' | 'dark';

export type Task = {
  id: string;
  card: 'anchor' | 'rotating' | 'extra';
  ask: string;
  start: string;
  goal: {path: string; hash?: string; text: RegExp};
  // The words a reader scans for, best first. The screen-only user clicks only visible controls
  // whose accessible name matches one of these.
  words: RegExp[];
};

export const site = {
  baseUrl: '/connectors',
  build: path.join(__dirname, '..', 'build'),
  port: Number(process.env.UX_PORT ?? 3141),
};

export const gatingViewports: Viewport[] = [
  {name: '1366x768', width: 1366, height: 768},
  {name: '1280x720', width: 1280, height: 720},
];

export const measuredViewports: Viewport[] = [
  {name: '1440x900', width: 1440, height: 900},
  {name: '1024x768', width: 1024, height: 768},
  {name: '768x1024', width: 768, height: 1024},
  {name: '390x844', width: 390, height: 844},
];

// Docs System ships two colour modes (styles/tokens.css: [data-theme='light'], [data-theme='dark']).
export const themes: Theme[] = ['light', 'dark'];

// Docusaurus shows the docs sidebar from 997 px on.
export const desktopWidth = 997;

// Navigation boxes that scroll on their own; the screen-only user scans them before the page.
// Controls that open the navigation drawer below the desktop width.
export const navOpeners = [/^toggle navigation bar$/i];

export const navScrollers = ['.theme-doc-toc-desktop .table-of-contents', 'aside.theme-doc-sidebar-container nav.menu'];

export type AlwaysVisible = {name: string; selector: string; where: 'all' | 'docs'; desktopOnly?: boolean};

export const alwaysVisible: AlwaysVisible[] = [
  {name: 'navbar', selector: 'nav.navbar', where: 'all'},
  {name: 'search', selector: '.navbar [role="search"], .navbar .DocSearch-Button, .navbar input[type="search"]', where: 'all'},
  {name: 'theme toggle', selector: '.navbar button[title*="dark and light mode"], .navbar button[aria-label*="dark and light mode"]', where: 'all', desktopOnly: true},
  {name: 'sidebar', selector: 'aside.theme-doc-sidebar-container nav.menu', where: 'docs', desktopOnly: true},
];

// An always-visible element the site cannot supply itself; the gate holds it as a known loss
// against its finding, and a test fails as soon as the shell starts to ship it.
export const knownAbsent: Record<string, string> = {search: 'UXF-001'};

// axe-core rules whose cause is the pinned shared theme, held as known losses against their
// finding; `knownAxeProbe` is a page where each still shows, so the gate notices the theme's fix.
export const knownAxe: Record<string, string> = {'link-in-text-block': 'UXF-002'};
export const knownAxeProbe = '/docs/getting-started';

// Gate 2 problems of the shared theme's layout, by element and state, held against their finding.
export const knownHidden: Record<string, string> = {'sidebar@end': 'UXF-003'};
export const knownHiddenProbe = '/docs/getting-started';

export const tasks: Task[] = [
  {
    id: 'install-command',
    card: 'anchor',
    ask: 'Find the command that installs a release binary of connectors.',
    start: '/',
    goal: {path: '/docs/getting-started', text: /install -m 0755 "connectors-\$version-\$target\/connectors"/},
    words: [/install/i, /^get started$/i, /getting started/i],
  },
  {
    id: 'onboarding-gitlab',
    card: 'anchor',
    ask: 'Find the onboarding page of the GitLab connector.',
    start: '/',
    goal: {path: '/docs/connectors/gitlab', text: /^GitLab$/},
    words: [/^gitlab$/i, /^connectors$/i, /^documentation$/i],
  },
  {
    id: 'exit-code-walk',
    card: 'rotating',
    ask: 'Find the exit code of `connectors invoke --walk` without `--out`.',
    start: '/',
    goal: {path: '/docs/reference/contracts/cli', text: /--walk without --out \(naming/},
    words: [/^walking pages$/i, /^local cli$/i, /^contracts$/i],
  },
  {
    id: 'onboarding-slack',
    card: 'extra',
    ask: 'Find the onboarding page of the Slack connector.',
    start: '/',
    goal: {path: '/docs/connectors/slack', text: /^Slack$/},
    words: [/^slack$/i, /^connectors$/i, /^documentation$/i],
  },
  {
    id: 'exit-status-older-binary',
    card: 'extra',
    ask: 'Find the exit status an older binary answers on a store a newer release wrote.',
    start: '/',
    goal: {path: '/docs/guides/upgrade-the-cli', text: /exit status 1, failure/},
    words: [/upgrade/i, /^guides$/i, /^documentation$/i],
  },
  {
    id: 'cli-invoke',
    card: 'extra',
    ask: 'Find the options of `connectors invoke` in the CLI reference.',
    start: '/',
    goal: {path: '/docs/reference/cli', hash: 'connectors-invoke', text: /^connectors invoke$/},
    words: [/^connectors invoke$/i, /^connectors cli$/i, /^reference$/i, /^documentation$/i],
  },
  {
    id: 'status-pod-exec',
    card: 'extra',
    ask: 'Find whether Kubernetes pod exec is shipped.',
    start: '/',
    goal: {path: '/docs/status', text: /^Kubernetes pod exec$/},
    words: [/^status$/i],
  },
];

// The page list: every route of the built sitemap, as a path under the base URL.
export function pages(): string[] {
  const xml = readFileSync(path.join(site.build, 'sitemap.xml'), 'utf8');
  const routes = [...xml.matchAll(/<loc>([^<]*)<\/loc>/g)].map((m) => new URL(m[1]).pathname);
  return [...new Set(routes.map((r) => r.replace(new RegExp('^' + site.baseUrl), '') || '/'))].sort();
}

// One page per template. The fast loop checks these and the tasks' pages; the whole sitemap is
// swept by the round (`npm run ux:round`), which the 120 s budget of the fast loop cannot hold
// (measured 2026-11-22: about 2 s per page and viewport, 171 pages).
export const templatePages = [
  '/',
  '/docs',
  '/docs/getting-started',
  '/docs/status',
  '/docs/connectors',
  '/docs/connectors/gitlab',
  '/docs/concepts/governed-invocation',
  '/docs/guides/upgrade-the-cli',
  '/docs/examples/follow-a-request',
  '/docs/examples/contract-exercises',
  '/docs/reference/cli',
  '/docs/reference/contracts/cli',
  '/docs/reference/contracts/model/domains/connectors-mutations',
  '/docs/reference/adapters/kubernetes',
];

// axe-core in the fast loop: the templates, with a smaller contract page for the largest one
// (axe took 10–30 s on /docs/reference/contracts/cli on 2026-11-22); the round runs the largest.
export const fastAxePages = templatePages.map((p) => (p === '/docs/reference/contracts/cli' ? '/docs/reference/contracts/clock' : p));

export const fastPages =() => [...new Set([...templatePages, ...tasks.map((t) => t.goal.path)])].sort();

export const isDocsPage =(route: string) => route.startsWith('/docs');

export const axeTags = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

// The axe rules of those tags whose result a colour mode can change.
export const colourRules = ['color-contrast', 'link-in-text-block'];
