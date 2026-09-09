import React from 'react';
import { useDoc } from '@docusaurus/plugin-content-docs/client';
import DocItemContent from '@theme/DocItem/Content';
import DocItemPaginator from '@theme/DocItem/Paginator';
import type { Props } from '@theme/DocItem/Layout';
import DocumentationFrame from '@site/src/components/DocumentationFrame';
export default function DocItemLayout({
  children
}: Props) {
  const {
    metadata,
    frontMatter,
    toc
  } = useDoc();
  return <DocumentationFrame title={metadata.title} wide={Boolean(frontMatter.hide_table_of_contents)} headings={toc.map(h => ({
    id: h.id,
    title: h.value.replace(/<[^>]+>/g, ''),
    level: h.level
  }))}>
    <DocItemContent>{children}</DocItemContent>
    <div data-pagefind-ignore><DocItemPaginator /></div>
  </DocumentationFrame>;
}
