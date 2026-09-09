import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import { CardGrid, ContentCard, SectionHeader } from '@beyond10x/docs-system/components';
import { SearchMetadata } from '../components/DocumentationFrame';
export default function Home() {
  return <Layout title="Independent integrations. Shared guarantees." description="Call GitLab, Kubernetes and SQL through independent adapter services with shared contracts for access, results and failures.">
    <main className="connectors-home" data-pagefind-body>
      <SearchMetadata title="Connectors — independent integrations" section="Introduction" owner="shared" kind="Guide" />
      <header className="home-hero">
        <div className="home-hero-inner">
          <div className="home-hero-copy"><p className="eyebrow">BEYOND10X / CONNECTORS</p><h1>Independent integrations.<br /><span>Shared guarantees.</span></h1>
            <p className="home-lede">Read issues from GitLab. Discover Kubernetes endpoints. Query a database. Independent adapters give your application a common way to call each service—and understand its results.</p>
            <div className="home-actions"><Link className="button button--primary" to="/introduction/examples">Follow a request to GitLab <span aria-hidden="true">→</span></Link><Link className="button button--outline" to="/introduction/getting-started">Run your first service</Link></div>
            <p className="home-preview">A working data and discovery slice, with a broader reviewed specification. <Link to="/introduction/status">See what works today →</Link></p>
          </div>
          <aside className="home-request" aria-label="A request through Connectors">
            <div className="home-request-title"><span>ONE ISSUE READ</span><span className="home-live-dot">GitLab</span></div>
            <ol><li><span className="request-number">01</span><div><strong>Your application</strong><span>Ask for issues in acme/website</span></div></li><li><span className="request-number">02</span><div><strong>A federating host</strong><span>Route to the configured remote adapter</span></div></li><li><span className="request-number">03</span><div><strong>The GitLab adapter</strong><span>Check the request, then call GitLab</span></div></li></ol>
            <div className="home-response"><span aria-hidden="true">↳</span><div><strong>Issues, with context</strong><span>Items · pagination · source provenance</span></div></div>
            <p>Call an adapter directly, or use a host when you need federation. Provider credentials stay at the adapter boundary.</p>
          </aside>
        </div>
      </header>
      <section className="home-section">
        <SectionHeader eyebrow="START WITH YOUR TASK" title="One place to understand the whole request." description="Get oriented, inspect a guarantee, or find the provider you need." />
        <CardGrid columns={3}>
          <ContentCard eyebrow="01 / INTRODUCTION" title="See how it works" description="Follow a practical request from a laptop, through a remote host, to GitLab—with authentication explained along the way." actionUrl="/introduction" actionLabel="Meet Connectors" />
          <ContentCard eyebrow="02 / CONTRACTS" title="Understand the guarantees" description="Read what an operation means: who can call it, how results are bounded, and what happens when a request fails." actionUrl="/contracts" actionLabel="Explore contracts" />
          <ContentCard eyebrow="03 / ADAPTERS" title="Choose an integration" description="Find provider-specific operations, native contracts and configuration. See which capabilities run today." actionUrl="/adapters" actionLabel="Browse adapters" />
        </CardGrid>
      </section>
      <section className="home-section home-working">
        <SectionHeader eyebrow="AVAILABLE TODAY" title="Start with a small, working service." description="The first runtime slice supports bounded reads and discovery. Each adapter documents its precise limits." action={<Link to="/introduction/status">Full support status →</Link>} />
        <div className="home-adapters">{[['GitLab', 'Projects, issues and files', 'gitlab'], ['Kubernetes', 'Resources and endpoints', 'kubernetes'], ['SQL / PostgreSQL', 'Schemas and read-only queries', 'sql']].map(([title, description, id]) => <Link key={id} to={'/adapters/' + id}><strong>{title}</strong><span>{description}</span><span aria-hidden="true">↗</span></Link>)}</div>
      </section>
    </main>
  </Layout>;
}
