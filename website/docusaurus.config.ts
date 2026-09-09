import type {Config} from '@docusaurus/types';
import type {Options, ThemeConfig} from '@docusaurus/preset-classic';
import {ecosystemFooterGroup} from '@beyond10x/docs-system/docusaurus';
import {PRISM_ADDITIONAL_LANGUAGES} from '@beyond10x/docs-system/code';
import {themes} from 'prism-react-renderer';

const config: Config = {
  title: 'Connectors',
  tagline: 'Independent integrations. Shared guarantees.',
  favicon: 'img/mark.svg',
  url: 'http://localhost:3100',
  baseUrl: '/',
  trailingSlash: false,
  staticDirectories: process.env.NODE_ENV==='development'?['static','.cache/search-public']:['static'],
  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',
  markdown: {mermaid: true, hooks: {onBrokenMarkdownLinks: 'throw'}},
  themes: ['@docusaurus/theme-mermaid'],
  plugins: ['./plugins/reference.ts', './plugins/search.ts'],
  i18n: {defaultLocale: 'en', locales: ['en']},
  presets: [['classic', {
    docs: {routeBasePath: '/', sidebarPath: './sidebars.ts'},
    blog: false,
    theme: {customCss: './src/css/custom.css'},
  } satisfies Options]],
  themeConfig: {
    colorMode: {defaultMode: 'light', respectPrefersColorScheme: true},
    navbar: {
      title: 'connectors',
      logo: {alt: '', src: 'img/mark.svg', width: 28, height: 28},
      items: [
        {to: '/introduction', label: 'Home / Introduction', position: 'left', activeBaseRegex: '^/introduction'},
        {to: '/contracts', label: 'Contracts', position: 'left', activeBaseRegex: '^/contracts'},
        {to: '/adapters', label: 'Adapters', position: 'left', activeBaseRegex: '^/adapters'},
        {to: '/search', label: 'Search', position: 'right'},
        {to: '/introduction/status', label: 'Current status', position: 'right'},
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {title: 'Start here', items: [{label: 'Introduction', to: '/introduction'}, {label: 'Follow a request', to: '/introduction/examples'}, {label: 'Run a service', to: '/introduction/getting-started'}]},
        {title: 'Documentation', items: [{label: 'Contracts', to: '/contracts'}, {label: 'Adapters', to: '/adapters'}, {label: 'Search documentation', to: '/search'}, {label: 'Current status', to: '/introduction/status'}]},
        ecosystemFooterGroup(),
      ],
      copyright: 'Connectors · beyond10x · Local documentation preview',
    },
    prism: {theme: themes.github, darkTheme: themes.dracula, additionalLanguages: [...PRISM_ADDITIONAL_LANGUAGES]},
  } satisfies ThemeConfig,
};
export default config;
