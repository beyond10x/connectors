import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';
import { mkdirSync, writeFileSync } from 'node:fs';
import fs from 'node:fs/promises';
import type { LoadContext, Plugin } from '@docusaurus/types';
const run = promisify(execFile);
export default function searchPlugin(context: LoadContext): Plugin {
  const preview = path.join(context.siteDir, '.cache/search-public/pagefind');
  mkdirSync(preview, {
    recursive: true
  });
  // Docusaurus refuses an empty static directory on a fresh preview.
  writeFileSync(path.join(preview, 'preview.txt'), 'Local search is refreshed by the documentation build.\n');
  async function audit(directory: string) {
    const {
      stdout
    } = await run('cargo', ['run', '--locked', '--offline', '-p', 'connectors-build', '--', 'docs-audit', '--directory', directory], {
      cwd: path.dirname(context.siteDir),
      env: {
        ...process.env,
        CARGO_BUILD_JOBS: '2'
      }
    });
    process.stdout.write(stdout);
  }
  return {
    name: 'connectors-search',
    async postBuild({
      outDir
    }) {
      // Inspect readable HTML before compression as well as every final emitted byte.
      await audit(outDir);
      const {
        stdout
      } = await run(process.execPath, [path.join(context.siteDir, 'node_modules/pagefind/lib/runner/bin.cjs'), '--site', outDir, '--root-selector', '[data-pagefind-body]', '--exclude-selectors', '.provenance,.document-context,.documentation-breadcrumbs,.model-diagram details,button,select,input,[data-pagefind-ignore]'], {
        cwd: context.siteDir
      });
      process.stdout.write(stdout);
      await audit(outDir);
      // Retain immutable hashed fragments for tabs using an earlier index. Publish
      // the entrypoint last; no static search cache is copied into production builds.
      const files = await fs.readdir(path.join(outDir, 'pagefind'), {
        recursive: true,
        withFileTypes: true
      });
      const regular = files.filter(file => file.isFile()).sort((a, b) => Number(a.name === 'pagefind-entry.json') - Number(b.name === 'pagefind-entry.json'));
      for (const file of regular) {
        const from = path.join(file.parentPath, file.name);
        const target = path.join(preview, path.relative(path.join(outDir, 'pagefind'), from));
        await fs.mkdir(path.dirname(target), {
          recursive: true
        });
        await fs.copyFile(from, target + '.next');
        await fs.rename(target + '.next', target);
      }
      console.log('website: local full-text search indexed and preview refreshed');
    }
  };
}
