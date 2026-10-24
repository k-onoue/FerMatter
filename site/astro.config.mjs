import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import { unified } from "@astrojs/markdown-remark";
import config from "./src/data/site.json" with { type: "json" };
import pages from "../user-docs/pages.json" with { type: "json" };
import { remarkGuideLinks } from "../scripts/lib/guide-links.mjs";

const socialImage = new URL(
  `${config.base.replace(/\/$/, "")}/assets/icon.png`,
  config.url,
).href;
const socialImageAlt = "FerMatter app icon featuring Pierre de Fermat";

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
      head: [
        { tag: "meta", attrs: { property: "og:image", content: socialImage } },
        {
          tag: "meta",
          attrs: { property: "og:image:type", content: "image/png" },
        },
        { tag: "meta", attrs: { property: "og:image:width", content: "1024" } },
        {
          tag: "meta",
          attrs: { property: "og:image:height", content: "1024" },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:alt", content: socialImageAlt },
        },
        { tag: "meta", attrs: { name: "twitter:card", content: "summary" } },
        { tag: "meta", attrs: { name: "twitter:image", content: socialImage } },
        {
          tag: "meta",
          attrs: { name: "twitter:image:alt", content: socialImageAlt },
        },
      ],
      customCss: ["./src/styles/site.css"],
      components: {
        Header: "./src/components/Header.astro",
        ThemeProvider: "./src/components/ThemeProvider.astro",
        ThemeSelect: "./src/components/ThemeToggle.astro",
        PageTitle: "./src/components/PageTitle.astro",
        Footer: "./src/components/Footer.astro",
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
    }),
  ],
});
