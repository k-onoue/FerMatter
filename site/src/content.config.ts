import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { docsSchema } from '@astrojs/starlight/schema';
import pages from '../../user-docs/pages.json';

export const collections = {
  docs: defineCollection({
    loader: glob({
      base: new URL('../../user-docs/', import.meta.url),
      pattern: pages,
      generateId: ({ entry }) => entry === 'README.md' ? 'guide' : `guide/${entry.replace(/\.md$/, '')}`,
    }),
    schema: docsSchema(),
  }),
};
