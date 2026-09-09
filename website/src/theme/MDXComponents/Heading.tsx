import React from 'react';
import Heading from '@theme/Heading';
import type { Props } from '@theme/MDXComponents/Heading';
export default function MDXHeading(props: Props) {
  return <Heading {...props} data-pagefind-weight={props.as === 'h1' ? '8' : undefined} />;
}
