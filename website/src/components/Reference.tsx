import React, {useState} from 'react';
import type {ReactNode} from 'react';
import Layout from '@theme/Layout';
import {CodeExample} from '@beyond10x/docs-system/components';
import Mermaid from '@theme/Mermaid';
import Heading from '@theme/Heading';
import useBrokenLinks from '@docusaurus/useBrokenLinks';
import Link from '@docusaurus/Link';
import {usePluginData} from '@docusaurus/useGlobalData';
import type {Inline,Block,ReferencePage,ReferenceIndex} from './reference-types';
import DocumentationFrame, {type PageHeading} from './DocumentationFrame';
import {friendly} from './documentation';

function text(nodes:Inline[]):string {return nodes.map(node=>'text' in node?(typeof node.text==='string'?node.text:text(node.text)):' ').join('');}
function Diagram({source,kind}:{source:string;kind:string}){
  const [open,setOpen]=useState(kind==='lifecycle');
  const title=kind==='lifecycle'?'Lifecycle':kind+' diagram';
  return <details className="model-diagram" open={open} onToggle={event=>setOpen(event.currentTarget.open)}>
    <summary>{title}</summary>
    {open&&<figure className="model-diagram-figure" aria-label={title}>
      <div className="model-diagram-viewport"><Mermaid value={source}/></div>
      <figcaption>A diagram describes the model; it does not prove runtime enforcement.</figcaption>
    </figure>}
    <details data-pagefind-ignore><summary>Diagram source</summary><pre>{source}</pre></details>
  </details>;
}
function Inlines({nodes,routes}:{nodes:Inline[];routes:Record<string,string>}):ReactNode {
  return nodes.map((node,i)=>{
    switch(node.inline){
      case 'text':return <React.Fragment key={i}>{node.text}</React.Fragment>;
      case 'code':return <code key={i}>{node.text}</code>;
      case 'strong':return <strong key={i}><Inlines nodes={node.text} routes={routes}/></strong>;
      case 'emphasis':return <em key={i}><Inlines nodes={node.text} routes={routes}/></em>;
      case 'break':return <br key={i}/>;
      case 'link':{
        const to=node.to;
        const href=(to.target==='page'||to.target==='anchor')&&to.page?routes[to.page]+(to.anchor?'#'+to.anchor:''):to.url;
        if(!href || href.startsWith('undefined') || (!href.startsWith('/')&&!href.startsWith('https://')&&!href.startsWith('http://')))throw new Error('Unresolved ESS reference target');
        return <Link key={i} to={href}><Inlines nodes={node.text} routes={routes}/></Link>;
      }
      default:throw new Error('Unsupported ESS inline node');
    }
  });
}
function Blocks({blocks,routes}:{blocks:Block[];routes:Record<string,string>}):ReactNode {
  return blocks.map((block,i)=>{
    switch(block.block){
      case 'prose':return <p key={i}><Inlines nodes={block.text} routes={routes}/></p>;
      case 'section':{
        const level=(`h${Math.max(2,Math.min(6,block.level))}`) as 'h2';
        return <section key={i}><Heading as={level} id={block.anchor}>{text(block.title)}</Heading><Blocks blocks={block.blocks} routes={routes}/></section>;
      }
      case 'table':return <div className="reference-table" key={i}><table><thead><tr>{block.columns.map((cell,j)=><th key={j}><Inlines nodes={cell} routes={routes}/></th>)}</tr></thead><tbody>{block.rows.map((row,j)=><tr key={j}>{row.map((cell,k)=><td key={k}><Inlines nodes={cell} routes={routes}/></td>)}</tr>)}</tbody></table></div>;
      case 'list':{
        const List=block.ordered?'ol':'ul';return <List key={i}>{block.items.map((item,j)=><li key={j}><Blocks blocks={item} routes={routes}/></li>)}</List>;
      }
      case 'diagram':return <Diagram key={i} source={block.source} kind={block.kind}/>;
      case 'code':return <CodeExample key={i} language={block.language??'text'}>{block.source}</CodeExample>;
      default:throw new Error('Unsupported ESS documentation block');
    }
  });
}
export default function Reference({page}:{page:ReferencePage}) {
  const brokenLinks=useBrokenLinks();
  page.headings?.forEach(heading=>brokenLinks.collectAnchor(heading.id));
  page.links?.filter(href=>href.startsWith('/')||href.startsWith('#')).forEach(href=>brokenLinks.collectLink(href.startsWith('#')?page.route+href:href));
  const index=usePluginData('connectors-reference') as ReferenceIndex;
  const model=page.kind==='model';
  function modelHeadings(blocks:Block[]):PageHeading[]{return blocks.flatMap(block=>block.block==='section'?[{id:block.anchor,title:text(block.title),level:Math.max(2,block.level)},...modelHeadings(block.blocks)]:[]);}
  const headings=model&&page.document?modelHeadings(page.document.blocks):page.headings;
  const digest=page.digest??page.document?.provenance.provenance.source_digest;
  return <Layout title={page.title} description={page.status}>
    <DocumentationFrame title={friendly(page.title)} headings={headings} reference>
        <h1 data-pagefind-weight="8">{friendly(page.title)}</h1><p className="reference-status">{page.status}</p>
        <details className="provenance" data-pagefind-ignore><summary>Source and interpretation</summary><p>{model?'This view renders the ESS documentation projection. Typed declarations and lifecycle diagrams do not prove runtime behavior.':'Normative text comes from the owning contract. Historical extraction tables are omitted; unpublished evidence links are shown as source notes.'}</p><p className="reference-source">{model?'Generated by ESS '+index.ess:'Rendered from canonical source'} · <code>{page.source}</code></p><p>Source digest: <code>{digest}</code></p></details>
        {model&&page.document?<Blocks blocks={page.document.blocks} routes={page.modelRoutes??{}}/>:<div className="canonical-contract" dangerouslySetInnerHTML={{__html:page.html??''}}/>}
    </DocumentationFrame>
  </Layout>;
}
