// L3 screen-only user (skill § 7) and gate 3, unmissable completion: every task reaches its target
// with completion_miss = 0 and false_complete = 0, at both gating viewports in the first colour
// mode; the anchor cards also in the other modes at the first viewport (a colour mode does not
// change the page's controls; the round runs every combination). The metrics go to the ratchet.
import {gatingViewports, navOpeners, navScrollers, site, tasks, themes} from '../../ux/adapter';
import {record} from './core/contract';
import {chooseTheme, expect, test} from './core/fixture';
import {play} from './core/screen-only';

const runs = (anchor: boolean) => gatingViewports.flatMap((vp, i) => (i === 0 && anchor ? themes : themes.slice(0, 1)).map((theme) => ({vp, theme})));

for (const task of tasks) {
  for (const {vp, theme} of runs(task.card === 'anchor')) {
    test(`screen-only ${task.id} ${vp.name} ${theme}`, async ({page, offsite}) => {
      await page.setViewportSize({width: vp.width, height: vp.height});
      await chooseTheme(page, theme);
      const run = await play(page, site.baseUrl, task, navScrollers, undefined, navOpeners);
      record('screen-only', `${task.id}@${vp.name}@${theme}`, {viewport: vp.name, theme, ...run});
      const trail = run.steps.map((s) => (s.action === 'click' ? `click "${s.control}" (${s.region})` : s.action)).join(' → ');
      expect(offsite).toEqual([]);
      expect(run.done, `${task.id}: ${trail}`).toBe(true);
      expect(run.metrics.completion_miss, `${task.id}: target under the navbar or off screen`).toBe(0);
      expect(run.metrics.false_complete, `${task.id}: ended on ${run.endPath}`).toBe(0);
    });
  }
}
