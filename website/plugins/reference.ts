import fs from 'node:fs/promises';
import path from 'node:path';
import type {LoadContext, Plugin} from '@docusaurus/types';
import type {ReferencePage} from '../src/components/reference-types';

export default function referencePlugin(context: LoadContext): Plugin<{pages: ReferencePage[]; ess: string}> {
  const input = path.join(context.siteDir, '.cache/reference.json');
  return {
    name: 'connectors-reference',
    getPathsToWatch: () => [input],
    async loadContent() {
      try {return JSON.parse(await fs.readFile(input, 'utf8'));}
      catch (error) {throw new Error('Generate the contract reference with npm run reference before starting the website.', {cause: error});}
    },
    async contentLoaded({content, actions}) {
      const {createData, addRoute, setGlobalData} = actions;
      setGlobalData({ess:content.ess,pages:content.pages.map(({route,title,owner,status,kind})=>({route,title,owner,status,kind}))});
      for (const page of content.pages) {
        const data=await createData(page.route.replaceAll('/','_')+'.json',JSON.stringify(page));
        // Docusaurus includes module names in its browser route registry.
        // Use its portable alias so those names never expose the checkout path.
        const modulePath='@generated/'+path.relative(context.generatedFilesDir,data).split(path.sep).join('/');
        addRoute({path:page.route,component:'@site/src/components/Reference.tsx',modules:{page:modulePath},exact:true});
      }
    },
  };
}
