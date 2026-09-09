import React, { type ReactNode } from 'react';
import type { Props } from '@theme/DocRoot/Layout';

// Authored and generated pages share DocumentationFrame rather than two sidebars.
export default function DocRootLayout({
  children
}: Props): ReactNode {
  return <>{children}</>;
}
