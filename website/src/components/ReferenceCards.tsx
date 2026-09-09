import React from 'react';
import Link from '@docusaurus/Link';
import {usePluginData} from '@docusaurus/useGlobalData';
import type {ReferenceIndex} from './reference-types';
import {CardGrid, ContentCard} from '@beyond10x/docs-system/components';
import {family, friendly} from './documentation';
export default function ReferenceCards({owner='shared',kind='contract'}:{owner?:string;kind?:'contract'|'model'}){
  const {pages}=usePluginData('connectors-reference') as ReferenceIndex;
  const selected=pages.filter(page=>page.owner===owner&&page.kind===kind);
  const groups=owner==='shared'&&kind==='contract'?Array.from(new Set(selected.map(family))):[''];
  return <div className="reference-cards">{groups.map(group=><section key={group}>{group&&<h3>{group}</h3>}<CardGrid>{selected.filter(page=>!group||family(page)===group).map(page=><ContentCard key={page.route} title={friendly(page.title)} titleUrl={page.route} description={page.status} actionUrl={page.route} actionLabel={kind==='model'?'Inspect model':'Read contract'} headingLevel={group?4:3}/>)}</CardGrid></section>)}</div>;
}
