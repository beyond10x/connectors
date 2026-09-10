import type { PageMeta } from './reference-types';
export const adapters = [{
  id: 'gitlab',
  title: 'GitLab',
  summary: 'Saved credentials, repository and CI reads, and pinned-head MR validation.',
  status: 'Local CLI and eleven reads',
  runtime: true
}, {
  id: 'kubernetes',
  title: 'Kubernetes',
  summary: 'Resource inventory and endpoint discovery with explicit reachability.',
  status: 'Working first slice',
  runtime: true
}, {
  id: 'sql',
  title: 'SQL / PostgreSQL',
  summary: 'Schema discovery and bounded, read-only relational queries.',
  status: 'Working first slice',
  runtime: true
}, {
  id: 'atlassian',
  title: 'Atlassian',
  summary: 'Native Jira and Confluence document profiles.',
  status: 'Specification only',
  runtime: false
}, {
  id: 'docker',
  title: 'Docker',
  summary: 'Container operations, multiplexed logs and explicit mutations.',
  status: 'Specification only',
  runtime: false
}, {
  id: 'grafana',
  title: 'Grafana',
  summary: 'Datasource discovery and mediated access to independent providers.',
  status: 'Specification only',
  runtime: false
}, {
  id: 'loki',
  title: 'Loki',
  summary: 'Bounded LogQL reads, streams and cursor semantics.',
  status: 'Specification only',
  runtime: false
}, {
  id: 'prometheus',
  title: 'Prometheus',
  summary: 'Instant and range PromQL queries with explicit series semantics.',
  status: 'Specification only',
  runtime: false
}, {
  id: 'alertmanager',
  title: 'Alertmanager',
  summary: 'Alert-record reads and native configuration.',
  status: 'Design only',
  runtime: false
}, {
  id: 'sip',
  title: 'SIP',
  summary: 'Dial behavior behind shared session and media contracts.',
  status: 'Specification only',
  runtime: false
}, {
  id: 'rtvbp',
  title: 'RTVBP',
  summary: 'Session and media transport with proof-bound authority.',
  status: 'Specification only',
  runtime: false
}, {
  id: 'catalog',
  title: 'Catalog',
  summary: 'Optional discovery and distribution of precompiled adapter bundles.',
  status: 'Design and shared contract',
  runtime: false
}] as const;
export type NavItem = {
  title: string;
  route?: string;
  children?: NavItem[];
};
export const friendly = (title: string) => title.replaceAll('_', ' ');
export function family(page: Pick<PageMeta, 'route' | 'kind'>): string {
  if (page.kind === 'model') return 'ESS models';
  if (page.route.startsWith('/contracts/auth/')) return 'Authentication and access';
  if (page.route.startsWith('/contracts/data/')) return 'Data reads';
  if (page.route.startsWith('/contracts/discovery/')) return 'Discovery and composition';
  if (['/contracts/sessions', '/contracts/media'].includes(page.route)) return 'Sessions and media';
  return 'Service and execution';
}
export function referenceGroups(pages: PageMeta[]): NavItem[] {
  const groups = new Map<string, NavItem[]>();
  for (const page of pages) {
    const label = family(page);
    const items = groups.get(label) ?? [];
    items.push({
      title: friendly(page.title),
      route: page.route
    });
    groups.set(label, items);
  }
  return Array.from(groups, ([title, children]) => ({
    title,
    children
  }));
}
export function navigation(pages: PageMeta[]): NavItem[] {
  return [{
    title: 'Home / Introduction',
    route: '/introduction',
    children: [{
      title: 'What is Connectors?',
      route: '/introduction'
    }, {
      title: 'Follow a request to GitLab',
      route: '/introduction/examples'
    }, {
      title: 'Run your first service',
      route: '/introduction/getting-started'
    }, {
      title: 'Current status',
      route: '/introduction/status'
    }]
  }, {
    title: 'Contracts',
    route: '/contracts',
    children: [{
      title: 'Understand the guarantees',
      route: '/contracts'
    }, ...referenceGroups(pages.filter(p => p.owner === 'shared' && p.kind === 'contract')), {
      title: 'ESS model reference',
      route: '/contracts/models',
      children: [{
        title: 'Read the typed model',
        route: '/contracts/models'
      }, ...pages.filter(p => p.owner === 'shared' && p.kind === 'model').map(p => ({
        title: friendly(p.title),
        route: p.route
      }))]
    }, {
      title: 'Advanced contract exercises',
      route: '/contracts/examples'
    }]
  }, {
    title: 'Adapters',
    route: '/adapters',
    children: [{
      title: 'Choose an integration',
      route: '/adapters'
    }, ...adapters.map(adapter => ({
      title: adapter.title,
      route: '/adapters/' + adapter.id,
      children: [{
        title: 'Overview and support',
        route: '/adapters/' + adapter.id
      }, ...pages.filter(p => p.owner === adapter.id).map(p => ({
        title: friendly(p.title),
        route: p.route
      }))]
    }))]
  }];
}
export function pageContext(route: string) {
  const section = route.startsWith('/adapters') ? 'Adapters' : route.startsWith('/contracts') ? 'Contracts' : 'Introduction';
  const owner = section === 'Adapters' ? route.split('/')[2] ?? 'shared' : 'shared';
  const kind = route.includes('/model/') ? 'Model' : route.includes('/contracts/') && !['/contracts/models', '/contracts/examples'].includes(route) ? 'Contract' : route.endsWith('/examples') ? 'Example' : 'Guide';
  return {
    section,
    owner,
    kind
  };
}
