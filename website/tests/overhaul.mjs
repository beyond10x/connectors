import assert from 'node:assert/strict';
import {chromium} from 'playwright';

const base=process.env.CONNECTORS_WEBSITE_URL??'http://127.0.0.1:3100';
const browser=await chromium.launch();
const page=await browser.newPage({viewport:{width:1440,height:1000}});
const errors=[];
page.on('pageerror',error=>errors.push(String(error)));
async function visit(route){
  await page.goto(base+route,{waitUntil:'networkidle'});
  assert.equal(await page.locator('#webpack-dev-server-client-overlay').count(),0);
  assert.equal(await page.locator('h1').count(),1,route+' must have one primary heading');
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),route+' overflows');
}
async function results(){await page.waitForFunction(()=>document.querySelector('.search-page')?.getAttribute('data-results-for')===location.search&&/pages? found|No matching/.test(document.querySelector('.search-status')?.textContent??''));}
try {
  await visit('/contracts/auth/connection#41-connection-viability-and-operation-eligibility');
  const navigation=page.locator('.documentation-sidebar');
  const active=navigation.locator('[aria-current="page"]');
  assert.equal(await active.innerText(),'Connections');
  assert(await active.isVisible());
  assert.equal(await navigation.locator('details[open]>summary').count(),2);
  await navigation.getByRole('link',{name:'Credential custody',exact:true}).click();
  await page.waitForURL(base+'/contracts/auth/custody');
  assert.equal(await navigation.locator('[aria-current="page"]').innerText(),'Credential custody');
  await visit('/contracts');
  for(const family of ['Service and execution','Authentication and access','Data reads','Discovery and composition','Sessions and media'])assert.equal(await page.getByRole('heading',{name:family,exact:true}).count(),1);
  await visit('/adapters');
  assert.equal(await page.locator('.adapter-catalog .b10x-content-card').count(),12);
  await page.getByRole('button',{name:/Available runtime/}).click();
  assert.equal(await page.locator('.adapter-catalog .b10x-content-card').count(),3);
  await page.getByLabel('Find an adapter',{exact:true}).fill('GitLab');
  assert.equal(await page.locator('.adapter-catalog .b10x-content-card').count(),1);
  await page.getByLabel('Find an adapter',{exact:true}).fill('no-such-provider');
  assert.match(await page.locator('.adapter-catalog').innerText(),/No adapters match/);
  await visit('/adapters/gitlab/contracts/reads');
  assert.match(await page.locator('.document-context').innerText(),/Adapter runtime: Local CLI and eleven reads/);
  assert.equal(await navigation.locator('[aria-current="page"]').innerText(),'GitLab bounded reads');

  await visit('/search?q=GitLab&owner=gitlab');await results();
  assert.equal(await page.getByLabel('Owner',{exact:true}).inputValue(),'gitlab');
  const gitlabContracts=['/adapters/gitlab/contracts/reads','/adapters/gitlab/contracts/ci','/adapters/gitlab/contracts/merge-requests','/adapters/gitlab/contracts/merge-validation'].sort();
  const resultRoutes=()=>page.locator('.search-results>li a').evaluateAll(links=>links.map(link=>link.getAttribute('href')).sort());
  assert.deepEqual(await resultRoutes(),[...gitlabContracts,'/adapters/gitlab'].sort());
  assert(await page.locator('.search-results mark').count()>0,'excerpts highlight the query');
  const guide=page.locator('.search-results').getByRole('link',{name:'GitLab',exact:true});
  assert.equal(await guide.getAttribute('href'),'/adapters/gitlab');
  await guide.click();await page.waitForURL(base+'/adapters/gitlab');
  await page.goBack({waitUntil:'networkidle'});await results();
  assert.equal(await page.getByLabel('Search all documentation',{exact:true}).inputValue(),'GitLab');
  await page.getByLabel('Document type',{exact:true}).selectOption('Contract');await results();
  assert.deepEqual(await resultRoutes(),gitlabContracts);
  await page.reload({waitUntil:'networkidle'});await results();
  assert.equal(await page.getByLabel('Document type',{exact:true}).inputValue(),'Contract');
  await page.getByLabel('Search all documentation',{exact:true}).fill('xxyyzznonexistent');await results();
  assert.match(await page.locator('.search-status').innerText(),/No matching pages/);
  await visit('/search?type=Model&owner=kubernetes');await results();
  assert.equal(await page.locator('.search-results>li').count(),7);
  await visit('/search?q=connection');await results();
  assert.equal(await page.locator('.search-results>li').count(),20);
  await page.getByRole('button',{name:'Show more results',exact:true}).click();
  await page.waitForFunction(()=>document.querySelectorAll('.search-results>li').length===40);
  await visit('/search');
  await page.getByLabel('Search all documentation',{exact:true}).focus();
  await page.keyboard.type('bounded');await results();
  assert.notEqual(await page.getByLabel('Search all documentation',{exact:true}).evaluate(el=>getComputedStyle(el.parentElement).outlineStyle),'none');

  for(const theme of ['light','dark']){
    await page.evaluate(mode=>{localStorage.setItem('theme',mode);document.documentElement.setAttribute('data-theme',mode);},theme);
    for(const width of [1440,1024,768,390]){
      await page.setViewportSize({width,height:900});
      for(const route of ['/','/contracts','/contracts/auth/connection','/adapters','/adapters/gitlab','/introduction/examples','/search?q=GitLab','/contracts/model/domains/connectors-artifact_provenance','/contracts/model/domains/connectors-mutations']){
        await visit(route);
        if(route.includes('/model/')){
          // A one-state lifecycle used to be enlarged sevenfold and cropped.
          // Check both that case and a branching lifecycle in every viewport/theme.
          await page.locator('.model-diagram svg').first().waitFor({state:'visible'});
          const diagrams=await page.locator('.model-diagram svg').evaluateAll(svgs=>svgs.map(svg=>{
            const frame=svg.closest('.model-diagram-viewport');
            const bounds=svg.getBoundingClientRect();
            const viewport=frame.getBoundingClientRect();
            return {
              scale:svg.getScreenCTM().a,
              visible:bounds.width>0&&bounds.height>0,
              contained:bounds.left>=viewport.left&&bounds.right<=viewport.right&&bounds.top>=viewport.top&&bounds.bottom<=viewport.bottom,
              height:bounds.height,
              scrolls:frame.scrollWidth>frame.clientWidth+1||frame.scrollHeight>frame.clientHeight+1,
            };
          }));
          for(const diagram of diagrams){
            assert(diagram.visible&&diagram.contained,route+' clips its diagram');
            assert(diagram.scale<=1.01,route+' enlarges the diagram beyond its natural size');
            assert(diagram.height<=540,route+' exceeds the available screen height');
            assert(!diagram.scrolls,route+' requires scrolling to see the initial diagram');
          }
        }
        assert(await page.evaluate(()=>{
          const style=getComputedStyle(document.documentElement);
          return style.getPropertyValue('--ifm-background-color').trim()===style.getPropertyValue('--b10x-color-canvas').trim();
        }),'shared components and shell must use the same canvas');
        assert.match(await page.locator('h1').evaluate(el=>getComputedStyle(el).fontFamily),/Inter|sans-serif/);
        assert(await page.locator('body').evaluate(el=>parseFloat(getComputedStyle(el).fontSize)>=16));
      }
    }
  }
  await page.setViewportSize({width:390,height:844});
  await visit('/contracts/auth/connection');
  await page.locator('.documentation-mobile-nav>summary').focus();await page.keyboard.press('Enter');
  const mobile=page.locator('.documentation-mobile-nav');
  assert(await mobile.getByRole('link',{name:'Credential custody',exact:true}).isVisible());
  await mobile.getByRole('link',{name:'Credential custody',exact:true}).click();
  await page.waitForURL(base+'/contracts/auth/custody');
  assert.equal(await mobile.getAttribute('open'),null,'mobile navigation closes after changing page');
  await page.locator('.mobile-contents>summary').click();
  assert(await page.locator('.mobile-contents nav').isVisible());
  // 200% zoom-equivalent reflow, then actual CSS zoom for fixed/sticky geometry.
  await page.setViewportSize({width:720,height:500});
  await visit('/adapters');await page.evaluate(()=>document.documentElement.style.zoom='2');
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'200% zoom overflows');
  await page.evaluate(()=>document.documentElement.style.zoom='');

  const missing=await browser.newPage();
  await missing.route('**/pagefind/pagefind.js',route=>route.fulfill({status:404,body:'Missing index'}));
  await missing.goto(base+'/search',{waitUntil:'networkidle'});
  await missing.getByText('Search is unavailable',{exact:true}).waitFor();
  assert.equal(await missing.getByRole('link',{name:'Adapters',exact:true}).count()>0,true);
  await missing.close();
  assert.deepEqual(errors,[]);
  console.log('UI/UX checks: shared navigation, all reference families, adapter filters, full-text search/filter restoration/pagination/deep links, mobile menus, themes, responsive layouts, lifecycle viewport fit, zoom, focus and missing search index passed.');
} finally {await browser.close();}
