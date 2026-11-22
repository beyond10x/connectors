import {existsSync} from 'node:fs';
import path from 'node:path';
import {defineConfig} from 'playwright/test';

// The ui-iteration-loop's browser checks (tests/ux/, data in ux/). They test the built site in
// build/ and never build it: run `npm run build` once first. `gate` is the fast loop of every
// merge (layout contract, axe-core, screen-only user, findings, ratchet); `measure` is the round
// command that writes ux/rounds/<UX_ROUND>/. tests/ux/serve.mjs serves build/ as GitHub Pages does.
const port = Number(process.env.UX_PORT ?? 3141);
const build = path.join(__dirname, 'build');
if (!existsSync(path.join(build, 'sitemap.xml'))) throw new Error('no built site in build/: run `npm run build` first');

export default defineConfig({
  testDir: 'tests/ux',
  outputDir: '.cache/ux/test-results',
  globalSetup: './tests/ux/setup.ts',
  timeout: 60_000,
  globalTimeout: 120_000,
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  reporter: [['list'], ['./tests/ux/slowest-reporter.ts']],
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    browserName: 'chromium',
    headless: true,
    serviceWorkers: 'block',
    trace: 'off',
  },
  webServer: {
    command: `node tests/ux/serve.mjs ${port}`,
    url: `http://127.0.0.1:${port}/connectors/`,
    reuseExistingServer: false,
    timeout: 60_000,
    stdout: 'ignore',
  },
  projects: [
    {name: 'gate', testMatch: /(layout|a11y|screen-only|findings)\.spec\.ts$/},
    {name: 'ratchet', testMatch: /ratchet\.spec\.ts$/, dependencies: ['gate']},
    {name: 'measure', testMatch: /measure\.spec\.ts$/},
    {name: 'round', testMatch: /round-report\.spec\.ts$/, dependencies: ['measure']},
  ],
});
