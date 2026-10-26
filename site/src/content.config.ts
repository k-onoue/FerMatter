import { defineCollection } from "astro:content";
import { z } from "astro/zod";
import { glob } from "astro/loaders";
import { docsSchema } from "@astrojs/starlight/schema";
import pages from "../../user-docs/pages.json";

export const collections = {
  download: defineCollection({
    loader: glob({
      base: new URL("./content/download/", import.meta.url),
      pattern: "*.md",
    }),
    schema: z.object({
      title: z.string(),
      description: z.string(),
      platform: z.string().optional(),
      requirements: z.string().optional(),
      availability: z.string().optional(),
    }),
  }),
  news: defineCollection({
    loader: glob({
      base: new URL("./content/news/", import.meta.url),
      pattern: "*.md",
    }),
    schema: z.object({
      title: z.string(),
      description: z.string(),
      date: z.coerce.date(),
      status: z.enum(["upcoming", "available"]),
    }),
  }),
  docs: defineCollection({
    loader: glob({
      base: new URL("../../user-docs/", import.meta.url),
      pattern: pages,
      generateId: ({ entry }) =>
        entry === "README.md" ? "guide" : `guide/${entry.replace(/\.md$/, "")}`,
    }),
    schema: docsSchema(),
  }),
};
