// axe-core in the page, from the installed package: no network.
import type {Page} from 'playwright/test';

export type AxeHit = {id: string; impact: string; help: string; count: number; nodes: {target: string; summary: string}[]};

const source = require.resolve('axe-core/axe.min.js');

export async function axe(page: Page, tags: string[], rules?: string[]): Promise<AxeHit[]> {
  if (!(await page.evaluate(() => 'axe' in window))) await page.addScriptTag({path: source});
  const result = await page.evaluate(async ({t, only}) => {
    const w = window as unknown as {axe: {run: (ctx: unknown, opts: unknown) => Promise<{violations: {id: string; impact: string; help: string; nodes: {target: string[]; failureSummary: string}[]}[]}>}};
    const r = await w.axe.run(document, {runOnly: only ? {type: 'rule', values: only} : {type: 'tag', values: t}, resultTypes: ['violations']});
    return r.violations.map((v) => ({id: v.id, impact: v.impact, help: v.help, count: v.nodes.length, nodes: v.nodes.slice(0, 8).map((n) => ({target: n.target.join(' '), summary: (n.failureSummary ?? '').split('\n').slice(0, 3).join(' ')}))}));
  }, {t: tags, only: rules ?? null});
  return result;
}

export const serious = (hits: AxeHit[]) => hits.filter((h) => h.impact === 'serious' || h.impact === 'critical');
