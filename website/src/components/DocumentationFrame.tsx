import React, { useEffect, useState, type ReactNode } from 'react';
import Link from '@docusaurus/Link';
import { useLocation } from '@docusaurus/router';
import { usePluginData } from '@docusaurus/useGlobalData';
import { navigation, pageContext, adapters, type NavItem } from './documentation';
import type { ReferenceIndex } from './reference-types';
export type PageHeading = {
  id: string;
  title: string;
  level: number;
};
const contains = (item: NavItem, path: string): boolean => item.route === path || Boolean(item.children?.some(child => contains(child, path)));
function NavigationItem({
  item,
  path,
  depth = 0
}: {
  item: NavItem;
  path: string;
  depth?: number;
}) {
  const selected = contains(item, path);
  if (item.children?.length) return <li><details key={path} open={selected} className={depth === 0 ? 'nav-section' : 'nav-group'}>
    <summary>{item.title}<span aria-hidden="true">⌄</span></summary>
    <ul>{item.children.map(child => <NavigationItem key={child.route ?? child.title} item={child} path={path} depth={depth + 1} />)}</ul>
  </details></li>;
  return <li><Link to={item.route!} aria-current={item.route === path ? 'page' : undefined}>{item.title}</Link></li>;
}
export function DocumentationNavigation() {
  const {
    pathname
  } = useLocation();
  const path = pathname.replace(/\/$/, '');
  const {
    pages
  } = usePluginData('connectors-reference') as ReferenceIndex;
  return <nav aria-label="Documentation" className="documentation-nav" data-pagefind-ignore>
    <Link className="documentation-search" to="/search"><span aria-hidden="true">⌕</span> Search documentation <span aria-hidden="true">↗</span></Link>
    <ul>{navigation(pages).map(item => <NavigationItem key={item.title} item={item} path={path} />)}</ul>
  </nav>;
}
function Contents({
  headings
}: {
  headings: PageHeading[];
}) {
  const [active, setActive] = useState('');
  useEffect(() => {
    const observer = new IntersectionObserver(entries => {
      const visible = entries.filter(e => e.isIntersecting);
      if (visible.length) setActive(visible[0].target.id);
    }, {
      rootMargin: '-80px 0px -65% 0px'
    });
    headings.forEach(h => {
      const node = document.getElementById(h.id);
      if (node) observer.observe(node);
    });
    return () => observer.disconnect();
  }, [headings]);
  return <nav aria-label="On this page" className="page-contents" data-pagefind-ignore><span>On this page</span><ul>{headings.filter(h => h.level <= 3).map(h => <li key={h.id} className={h.level === 3 ? 'contents-nested' : undefined}><Link to={'#' + h.id} aria-current={active === h.id ? 'location' : undefined}>{h.title}</Link></li>)}</ul></nav>;
}
export function SearchMetadata({
  title,
  section,
  owner,
  kind
}: {
  title: string;
  section: string;
  owner: string;
  kind: string;
}) {
  return <div hidden aria-hidden="true" data-pagefind-ignore><span data-pagefind-meta="title">{title}</span><span data-pagefind-filter="section">{section}</span><span data-pagefind-filter="owner">{owner}</span><span data-pagefind-filter="type">{kind}</span></div>;
}
export default function DocumentationFrame({
  title,
  headings = [],
  wide = false,
  children,
  reference = false
}: {
  title: string;
  headings?: PageHeading[];
  wide?: boolean;
  children: ReactNode;
  reference?: boolean;
}) {
  const {
    pathname
  } = useLocation();
  const {
    section,
    owner,
    kind
  } = pageContext(pathname);
  const adapter = adapters.find(a => a.id === owner);
  const base = '/' + (section === 'Introduction' ? 'introduction' : section.toLowerCase());
  return <div className={'documentation-frame' + (wide ? ' documentation-wide' : '')}>
    <aside className="documentation-sidebar"><DocumentationNavigation /></aside>
    <main id="documentation-content" className="documentation-main">
      <details className="documentation-mobile-nav" key={pathname} data-pagefind-ignore><summary>Browse documentation</summary><DocumentationNavigation /></details>
      <div className="documentation-page">
        <article className={reference ? 'reference-article markdown' : 'documentation-article'} data-pagefind-body>
          <SearchMetadata title={title} section={section} owner={owner} kind={kind} />
          <nav aria-label="Breadcrumbs" className="documentation-breadcrumbs" data-pagefind-ignore><Link to="/">Connectors</Link><span>/</span><Link to={base}>{section}</Link>{adapter && <><span>/</span><Link to={'/adapters/' + adapter.id}>{adapter.title}</Link></>}</nav>
          <div className="document-context" data-pagefind-ignore><span>{kind}</span>{adapter && <span className={adapter.runtime ? 'support-working' : 'support-planned'}>{reference ? 'Adapter runtime: ' : ''}{adapter.status}</span>}<Link to="/introduction/status">Support and limits ↗</Link></div>
          {headings.length > 0 && !wide && <details className="mobile-contents" data-pagefind-ignore><summary>On this page</summary><Contents headings={headings} /></details>}
          {children}
        </article>
        {!wide && headings.length > 0 && <aside className="documentation-toc"><Contents headings={headings} /></aside>}
      </div>
    </main>
  </div>;
}
