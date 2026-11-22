import type {PluginOptions} from '@docusaurus/plugin-content-docs';

type Generator = PluginOptions['sidebarItemsGenerator'];
type Item = Awaited<ReturnType<Generator>>[number];

export const specificationPage = /^(reference\/(contracts|crates)(\/|$)|reference\/adapters\/[^/]+\/(contracts|model)(\/|$)|concepts\/contracts$|guides\/build-a-catalog-bundle$)/;
export const specificationDir = /^reference\/(contracts|adapters\/[^/]+\/(contracts|model))$/;

function withoutSpecification(items: Item[]): Item[] {
  const kept: Item[] = [];
  for (const item of items) {
    if (item.type === 'doc' && specificationPage.test(item.id)) continue;
    if (item.type === 'category') {
      const children = withoutSpecification(item.items);
      const link = item.link?.type === 'doc' && specificationPage.test(item.link.id) ? undefined : item.link;
      if (children.length === 0 && !link) continue;
      kept.push({...item, items: children, link});
      continue;
    }
    kept.push(item);
  }
  return kept;
}

export const sidebarItemsGenerator: Generator = async (args) => {
  const items = await args.defaultSidebarItemsGenerator(args);
  return specificationDir.test(args.item.dirName) ? items : withoutSpecification(items as Item[]);
};
