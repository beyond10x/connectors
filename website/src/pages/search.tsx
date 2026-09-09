import React, { useEffect, useState } from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import { useHistory, useLocation } from '@docusaurus/router';
import { PageHeader, SearchField, Callout } from '@beyond10x/docs-system/components';
type Filters = {
  section: string;
  type: string;
  owner: string;
};
type Result = {
  url: string;
  excerpt: string;
  meta: {
    title?: string;
  };
  filters: Partial<Record<keyof Filters, string[]>>;
};
type SearchEngine = {
  search: (query: string | null, options: {
    filters: Partial<Filters>;
  }) => Promise<{
    results: {
      data: () => Promise<Result>;
    }[];
  }>;
  filters: () => Promise<Record<keyof Filters, Record<string, number>>>;
};
const keys = ['section', 'type', 'owner'] as const;
const labels = {
  section: 'Section',
  type: 'Document type',
  owner: 'Owner'
};
const empty = {
  section: '',
  type: '',
  owner: ''
};
const importSearch = new Function('url', 'return import(url)') as (url: string) => Promise<SearchEngine>;
function publicRoute(url: string) {
  const parsed = new URL(url, 'https://connectors.invalid');
  return parsed.pathname.replace(/\/index\.html$/, '/').replace(/\.html$/, '') + parsed.hash;
}

// Pagefind excerpts contain mark elements. Render only their text and highlighting;
// never inject the search engine's HTML into the page.
function Excerpt({
  html
}: {
  html: string;
}) {
  const nodes: React.ReactNode[] = [];
  const document = new DOMParser().parseFromString(html, 'text/html');
  function visit(node: Node): React.ReactNode {
    if (node.nodeType === Node.TEXT_NODE) return node.textContent;
    const children = Array.from(node.childNodes).map((child, i) => <React.Fragment key={i}>{visit(child)}</React.Fragment>);
    return node.nodeName === 'MARK' ? <mark>{children}</mark> : <>{children}</>;
  }
  document.body.childNodes.forEach((node, i) => nodes.push(<React.Fragment key={i}>{visit(node)}</React.Fragment>));
  return <>{nodes}</>;
}
export default function Search() {
  const location = useLocation();
  const history = useHistory();
  const params = new URLSearchParams(location.search);
  const query = params.get('q') ?? '';
  const filters = Object.fromEntries(keys.map(key => [key, params.get(key) ?? ''])) as Filters;
  const [engine, setEngine] = useState<SearchEngine>();
  const [available, setAvailable] = useState<Record<string, Record<string, number>>>({});
  const [results, setResults] = useState<Result[]>([]);
  const [count, setCount] = useState(0);
  const [limit, setLimit] = useState(20);
  const [state, setState] = useState<'loading' | 'ready' | 'searching' | 'unavailable'>('loading');
  const [completedQuery, setCompletedQuery] = useState<string>();
  useEffect(() => {
    let current = true;
    importSearch('/pagefind/pagefind.js').then(async module => {
      const values = await module.filters();
      if (current) {
        setEngine(module);
        setAvailable(values);
        setState('ready');
      }
    }).catch(() => {
      if (current) setState('unavailable');
    });
    return () => {
      current = false;
    };
  }, []);
  useEffect(() => {
    setLimit(20);
  }, [location.search]);
  const hasQuery = query.trim().length >= 2 || keys.some(key => filters[key]);
  useEffect(() => {
    if (!engine) return;
    let current = true;
    setResults([]);
    setCount(0);
    setCompletedQuery(undefined);
    if (!hasQuery) {
      setState('ready');
      return;
    }
    setState('searching');
    const timer = setTimeout(() => {
      const selected = Object.fromEntries(keys.filter(key => filters[key]).map(key => [key, filters[key]]));
      engine.search(query.trim().length >= 2 ? query.trim() : null, {
        filters: selected
      }).then(async result => {
        const data = await Promise.all(result.results.slice(0, limit).map(r => r.data()));
        if (current) {
          setResults(data);
          setCount(result.results.length);
          setState('ready');
          setCompletedQuery(location.search);
        }
      }).catch(() => {
        if (current) setState('unavailable');
      });
    }, 150);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [engine, location.search, limit]);
  function update(nextQuery: string, nextFilters: Filters) {
    const next = new URLSearchParams();
    if (nextQuery) next.set('q', nextQuery);
    for (const key of keys) if (nextFilters[key]) next.set(key, nextFilters[key]);
    history.replace({
      pathname: '/search',
      search: next.toString() ? '?' + next.toString() : ''
    });
  }
  return <Layout title="Search documentation" description="Search Connectors guides, contracts, ESS models and adapter documentation.">
    <main className="search-page" data-pagefind-ignore aria-busy={state === 'searching' || state === 'loading'} data-results-for={completedQuery}>
      <PageHeader eyebrow="CONNECTORS / DOCUMENTATION" title="Find the answer you need." description="Search guides, shared contracts, typed models and provider-specific documentation." />
      <div className="search-controls"><SearchField label="Search all documentation" placeholder="connection, GitLab issues, bounded reads…" value={query} onChange={e => update(e.target.value, filters)} />
        <div className="search-filters">{keys.map(key => <label key={key}>{labels[key]}<select aria-label={labels[key]} value={filters[key]} onChange={e => update(query, {
              ...filters,
              [key]: e.target.value
            })}><option value="">All</option>{filters[key] && !available[key]?.[filters[key]] && <option value={filters[key]}>{filters[key]}</option>}{Object.keys(available[key] ?? {}).sort().map(value => <option key={value} value={value}>{value === 'shared' ? 'Shared / Connectors' : value}</option>)}</select></label>)}</div>
        {keys.some(key => filters[key]) && <div><button className="button button--outline button--primary" onClick={() => update(query, empty)}>Clear filters</button></div>}
      </div>
      {state === 'unavailable' ? <Callout tone="warning" title="Search is unavailable">The local search index could not be loaded. Reload to try again, or browse <Link to="/contracts">Contracts</Link> and <Link to="/adapters">Adapters</Link>.</Callout> : <>
        <p className="search-status" role="status">{state === 'loading' ? 'Loading search…' : state === 'searching' ? 'Searching…' : !hasQuery ? 'Enter at least two characters, or choose a filter.' : count === 0 ? 'No matching pages. Try another phrase or clear a filter.' : `${count} ${count === 1 ? 'page' : 'pages'} found${count > limit ? ` · showing ${limit}` : ''}`}</p>
        <ol className="search-results">{results.map(result => <li key={result.url}><div className="search-result-meta">{keys.map(key => <span key={key}>{result.filters[key]?.join(', ')}</span>)}</div><h2><Link to={publicRoute(result.url)}>{result.meta.title ?? result.url}</Link></h2><p><Excerpt html={result.excerpt} /></p></li>)}</ol>
        {count > limit && <button className="button button--outline button--primary" onClick={() => setLimit(value => value + 20)}>Show more results</button>}
      </>}
    </main>
  </Layout>;
}
