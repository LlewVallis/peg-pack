// @ts-check

const lightCodeTheme = require("prism-react-renderer/themes/github");
const darkCodeTheme = require("prism-react-renderer/themes/dracula");

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: "Peg Pack",
  tagline: "Versatile parser generator for PEG grammars",
  url: "https://peg-pack.netlify.app",
  baseUrl: "/",
  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "warn",
  favicon: "img/logo.png",
  organizationName: "LlewVallis",
  projectName: "peg-pack",
  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },
  presets: [
    [
      "classic",
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve("./sidebars.js"),
          routeBasePath: "docs",
          path: "docs",
          editUrl: "https://github.com/LlewVallis/peg-pack/tree/master/docs",
        },
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      navbar: {
        title: "Peg Pack",
        logo: {
          alt: "Peg Pack Logo",
          src: "img/logo.png",
        },
        items: [
          {
            to: "/docs/guide/background",
            label: "Guide",
            position: "left",
            activeBaseRegex: "^/docs/guide",
          },
          {
            to: "/docs/reference",
            label: "Reference",
            position: "left",
            activeBaseRegex: "^/docs/reference",
          },
          {
            to: "/docs/features",
            label: "Features",
            position: "left",
            activeBaseRegex: "^/docs/features",
          },
          {
            to: "/docs/security",
            label: "Security",
            position: "left",
            activeBaseRegex: "^/docs/security",
          },
          {
            href: "pathname:///rustdoc/parser",
            label: "Rustdoc",
            position: "right",
          },
          {
            href: "https://github.com/LlewVallis/peg-pack",
            label: "GitHub",
            position: "right",
          },
        ],
      },
      footer: {
        copyright: "Built by <a target='_blank' href='https://llew.netlify.app/'>Llew Vallis</a> :)",
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
        additionalLanguages: ["rust"],
      },
    }),
};

module.exports = config;
