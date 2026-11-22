import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {join, dirname} from 'node:path';
import {fileURLToPath} from 'node:url';

// Static check of the built docs sidebar (run `npm run build` first). The contract and model pages
// sit in one collapsed "Specification" category; the CLI reference and the hand-written adapter
// pages stay under "Reference"; the shared guides stay under "Guides". With a recorded route list
// as the first argument (one sitemap <loc> per line, sorted) it also asserts that no built route
// moved.
const build=join(dirname(fileURLToPath(import.meta.url)),'..','build');
const prefix='/connectors';
const marker='<li class="theme-doc-sidebar-item-category theme-doc-sidebar-item-category-level-1';
const read=file=>readFileSync(join(build,file),'utf8').split(marker).slice(1).map(block=>({
  label:/categoryLinkLabel[^>]*>([^<]+)</.exec(block)?.[1],
  collapsed:/aria-expanded="false"/.test(block.slice(0,600)),
  links:[...block.matchAll(/href="([^"]+)"/g)].map(m=>m[1].replace(prefix,'').replace(/\/$/,'')),
}));
const cats=read('docs/reference/cli/index.html');
const named=label=>cats.find(c=>c.label===label);
const spec=named('Specification');
assert(spec,'a Specification category exists; found '+cats.map(c=>c.label).join(', '));
assert(spec.collapsed,'Specification is collapsed');
const ref=named('Reference');
const guides=named('Guides');
const concepts=named('Concepts');
for(const route of [
  '/docs/reference/contracts/auth/connection',
  '/docs/reference/crates',
  '/docs/concepts/contracts',
  '/docs/guides/build-a-catalog-bundle',
  '/docs/reference/adapters/catalog/contracts/feed',
  '/docs/reference/adapters/prometheus/model',
]){
  const shown=read('docs'+route.slice(5)+'/index.html');
  assert(!shown.some(c=>c.label==='Reference'&&c.links.includes(route)),route+' is not under Reference');
  assert(shown.find(c=>c.label==='Specification')?.links.includes(route),route+' is under Specification');
}
for(const p of ['/docs/reference/cli','/docs/reference/adapters'])
  assert(ref.links.includes(p),p+' stays under Reference');
assert(read('docs/reference/adapters/gitlab/index.html').find(c=>c.label==='Reference').links.includes('/docs/reference/adapters/gitlab'),'the GitLab adapter page stays under Reference');
const specific=/^\/docs\/reference\/(contracts|crates)|^\/docs\/reference\/adapters\/[^/]+\/(contracts|model)/;
for(const l of ref.links)assert(!specific.test(l),l+' must not be under Reference');
for(const g of ['first-read','guarded-write','reconnect-a-connection','google-oauth-setup','pinned-instances'])
  assert(guides.links.includes('/docs/guides/'+g),g+' stays under Guides');
assert(!guides.links.includes('/docs/guides/build-a-catalog-bundle'),'build-a-catalog-bundle leaves Guides');
assert(!concepts.links.slice(1).includes('/docs/concepts/contracts'),'concepts/contracts leaves Concepts');
const recorded=process.argv[2];
if(recorded){
  const now=[...readFileSync(join(build,'sitemap.xml'),'utf8').matchAll(/<loc>([^<]*)/g)].map(m=>m[1]).sort();
  const before=readFileSync(recorded,'utf8').split('\n').filter(Boolean).sort();
  assert.deepEqual(now,before,'the built routes are the recorded routes');
}
console.log('Sidebar checks: Specification collapsed with the contract and model pages, Reference and Guides membership'+(recorded?', routes unchanged':'')+' passed.');
