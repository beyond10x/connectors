import {rmSync} from 'node:fs';
import {resultsDir} from './core/contract';

// Results are this run's only: the ratchet must never read a value an earlier run left behind.
export default function setup(): void {
  // UX_KEEP=1 adds to the results of an earlier run: a round split into several runs.
  if (process.env.UX_KEEP !== '1') rmSync(resultsDir, {recursive: true, force: true});
}
