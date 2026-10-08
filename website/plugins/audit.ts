import {execFile} from 'node:child_process';
import path from 'node:path';
import {promisify} from 'node:util';
import type {LoadContext, Plugin} from '@docusaurus/types';

const run = promisify(execFile);

// After every production build, the Rust public-output audit reads every emitted file,
// WASM included, and fails the build on a workstation, `.local/` or `.engineering/` path.
export default function auditPlugin(context: LoadContext): Plugin {
  return {
    name: 'connectors-public-audit',
    async postBuild({outDir}) {
      const {stdout} = await run(
        'cargo',
        ['run', '--locked', '--offline', '-q', '-p', 'connectors-build', '--', 'docs-audit', '--directory', outDir],
        {cwd: path.dirname(context.siteDir), env: {...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? '2'}},
      );
      process.stdout.write(stdout);
    },
  };
}
