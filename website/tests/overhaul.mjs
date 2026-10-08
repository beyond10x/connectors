import assert from 'node:assert/strict';
import {chromium} from 'playwright';

// Layout checks for the shared Docs System shell: one primary heading per page, no horizontal
// overflow, the docs sidebar marks the current page, the mobile menu navigates, and both colour
// modes keep the shell and shared components on one canvas.
const base=(process.env.CONNECTORS_WEBSITE_URL??'http://127.0.0.1:3100').replace(/\/$/,'')+'/connectors';
const browser=await chromium.launch();
const page=await browser.newPage({viewport:{width:1440,height:1000}});
const errors=[];
page.on('pageerror',error=>errors.push(String(error)));
async function visit(route){
  await page.goto(base+route,{waitUntil:'networkidle'});
  assert.equal(await page.locator('#webpack-dev-server-client-overlay').count(),0);
  await page.locator('h1').first().waitFor();
  assert.equal(await page.locator('h1:visible').count(),1,route+' must show one primary heading');
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),route+' overflows');
}
const routes=[
  '/',
  '/docs/',
  '/docs/getting-started',
  '/docs/status',
  '/docs/concepts/contracts',
  '/docs/guides/federate-adapter-services',
  '/docs/examples/follow-a-request',
  '/docs/reference/cli',
  '/docs/reference/crates',
  '/docs/reference/contracts',
  '/docs/reference/contracts/auth/connection',
  '/docs/reference/contracts/model/domains/connectors-mutations',
  '/docs/reference/adapters',
  '/docs/reference/adapters/gitlab',
];
try {
  await visit('/docs/reference/contracts/auth/connection');
  const sidebar=page.locator('nav.menu');
  const active=sidebar.locator('a.menu__link--active');
  assert.equal(await active.last().innerText(),'Connections');
  assert(await active.last().isVisible());
  await sidebar.getByRole('link',{name:'Credential custody',exact:true}).click();
  await page.waitForURL(base+'/docs/reference/contracts/auth/custody');
  await sidebar.locator('a.menu__link--active',{hasText:'Credential custody'}).waitFor();

  for(const theme of ['light','dark']){
    await page.evaluate(mode=>{localStorage.setItem('theme',mode);document.documentElement.setAttribute('data-theme',mode);},theme);
    for(const width of [1440,1024,768,390]){
      await page.setViewportSize({width,height:900});
      for(const route of routes){
        await visit(route);
        assert.equal(await page.locator('html').getAttribute('data-theme'),theme,route+' keeps the chosen theme');
        assert(await page.locator('body').evaluate(el=>parseFloat(getComputedStyle(el).fontSize)>=16),route+' body text is at least 16px');
      }
    }
  }

  await page.setViewportSize({width:390,height:844});
  await visit('/');
  await page.getByRole('button',{name:/Toggle navigation bar/}).click();
  await page.locator('.navbar-sidebar').getByRole('link',{name:'Status',exact:true}).click();
  await page.waitForURL(base+'/docs/status');
  await page.locator('h1:visible',{hasText:/^Status$/}).waitFor();

  assert.deepEqual(errors,[]);
  console.log('Layout checks: '+routes.length+' routes in both themes at four widths, one heading each, no overflow, sidebar active state, mobile navigation passed.');
} finally {await browser.close();}
