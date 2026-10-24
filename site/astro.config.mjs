import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import { unified } from "@astrojs/markdown-remark";
import config from "./src/data/site.json" with { type: "json" };
import pages from "../user-docs/pages.json" with { type: "json" };
import { remarkGuideLinks } from "../scripts/lib/guide-links.mjs";

export default defineConfig({
  site: config.url,
  base: config.base || "/",
  trailingSlash: "always",
  output: "static",
  markdown: {
    processor: unified({
      remarkPlugins: [[remarkGuideLinks, { base: config.base }]],
    }),
  },
  integrations: [
    starlight({
      title: config.name,
      description:
        "Browse, edit, and export LaTeX and Typst equations and diagrams.",
      logo: {
        light: "./public/assets/fermat-light.svg",
        dark: "./public/assets/fermat-dark.svg",
      },
      favicon: "/assets/favicon.svg",
      customCss: ["./src/styles/site.css"],
      components: {
        Header: "./src/components/Header.astro",
        ThemeProvider: "./src/components/ThemeProvider.astro",
        ThemeSelect: "./src/components/ThemeToggle.astro",
        PageTitle: "./src/components/PageTitle.astro",
        Footer: "./src/components/Footer.astro",
        EditLink: "./src/components/EditLink.astro",
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: `https://github.com/${config.repository}`,
        },
      ],
      sidebar: [
        {
          label: "Start here",
          items: pages.map((page) =>
            page === "README.md"
              ? "guide"
              : `guide/${page.replace(/\.md$/, "")}`,
          ),
        },
        {
          label: "Reported issues",
          items: [
            {
              label: "Welcome any trouble reports",
              link: `https://github.com/${config.repository}/issues`,
            },
          ],
        },
      ],
      credits: false,
      editLink: {
        baseUrl: `https://github.com/${config.repository}/edit/main/`,
      },
    }),
  ],
});
