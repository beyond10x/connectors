import React, { useState } from 'react';
import { CardGrid, ContentCard, SearchField, FilterChipGroup } from '@beyond10x/docs-system/components';
import { adapters } from './documentation';
export default function AdapterCatalog() {
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState('all');
  const results = adapters.filter(a => (filter === 'all' || (filter === 'working' ? a.runtime : !a.runtime)) && (a.title + ' ' + a.summary).toLowerCase().includes(query.trim().toLowerCase()));
  return <div className="adapter-catalog">
    <div className="catalog-controls" data-pagefind-ignore><SearchField label="Find an adapter" placeholder="GitLab, logs, discovery…" value={query} onChange={e => setQuery(e.target.value)} /><FilterChipGroup label="Runtime availability" options={[{
        value: 'all',
        label: 'All adapters',
        count: adapters.length
      }, {
        value: 'working',
        label: 'Available runtime',
        count: 3
      }, {
        value: 'documented',
        label: 'Documented direction',
        count: adapters.length - 3
      }]} selected={[filter]} onToggle={setFilter} /></div>
    <p className="catalog-count" role="status" data-pagefind-ignore>{results.length} {results.length === 1 ? 'adapter' : 'adapters'}</p>
    <CardGrid>{results.map(a => <ContentCard key={a.id} title={a.title} titleUrl={'/adapters/' + a.id} description={a.summary} meta={<span className={a.runtime ? 'support-working' : 'support-planned'}>{a.status}</span>} actionUrl={'/adapters/' + a.id} actionLabel="Explore adapter" />)}</CardGrid>
    {results.length === 0 && <p>No adapters match. Try a provider name or clear the filters.</p>}
  </div>;
}
