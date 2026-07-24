import { defineConfig } from '@rspress/core';
import path from 'node:path';

const docsBase = process.env.DOCS_BASE ?? '/';
const normalizedBase = docsBase.endsWith('/') ? docsBase : `${docsBase}/`;
const withBase = (assetPath: string) => `${normalizedBase}${assetPath.replace(/^\//, '')}`;
const publicAssetOrigin = 'https://isolarail.ivanli.cc';
const socialPreviewUrl = `${publicAssetOrigin}/isolarail-social-preview.png`;

const zhNav = [
  { text: '快速开始', link: '/zh/start/quick-start' },
  { text: '硬件', link: '/zh/hardware/topology' },
  { text: '固件', link: '/zh/firmware/boot-runtime' },
  { text: '音效', link: '/zh/firmware/buzzer-audio-preview' },
  { text: '控制面', link: '/zh/control-plane/interfaces' },
  { text: '规格索引', link: '/zh/reference/specs' },
];

const enNav = [
  { text: 'Quick Start', link: '/en/start/quick-start' },
  { text: 'Hardware', link: '/en/hardware/topology' },
  { text: 'Firmware', link: '/en/firmware/boot-runtime' },
  { text: 'Audio', link: '/en/firmware/buzzer-audio-preview' },
  { text: 'Control Plane', link: '/en/control-plane/interfaces' },
  { text: 'Specs', link: '/en/reference/specs' },
];

const zhSidebar = [
  {
    text: '开始',
    items: [
      { text: '文档首页', link: '/zh/' },
      { text: '快速开始', link: '/zh/start/quick-start' },
    ],
  },
  {
    text: '硬件',
    items: [{ text: '硬件拓扑', link: '/zh/hardware/topology' }],
  },
  {
    text: '固件',
    items: [
      { text: '启动与运行期', link: '/zh/firmware/boot-runtime' },
      { text: '蜂鸣器音效预览', link: '/zh/firmware/buzzer-audio-preview' },
    ],
  },
  {
    text: '控制面',
    items: [{ text: '接口与本机工具', link: '/zh/control-plane/interfaces' }],
  },
  {
    text: '仪表板',
    items: [{ text: '前面板显示', link: '/zh/dashboard/front-panel' }],
  },
  {
    text: '参考',
    items: [{ text: '规格索引', link: '/zh/reference/specs' }],
  },
];

const enSidebar = [
  {
    text: 'Start',
    items: [
      { text: 'Docs Home', link: '/en/' },
      { text: 'Quick Start', link: '/en/start/quick-start' },
    ],
  },
  {
    text: 'Hardware',
    items: [{ text: 'Hardware Topology', link: '/en/hardware/topology' }],
  },
  {
    text: 'Firmware',
    items: [
      { text: 'Boot and Runtime', link: '/en/firmware/boot-runtime' },
      { text: 'Buzzer Audio Preview', link: '/en/firmware/buzzer-audio-preview' },
    ],
  },
  {
    text: 'Control Plane',
    items: [{ text: 'Interfaces and Tools', link: '/en/control-plane/interfaces' }],
  },
  {
    text: 'Dashboard',
    items: [{ text: 'Front Panel', link: '/en/dashboard/front-panel' }],
  },
  {
    text: 'Reference',
    items: [{ text: 'Specs Index', link: '/en/reference/specs' }],
  },
];

export default defineConfig({
  root: 'docs',
  base: normalizedBase,
  builderConfig: {
    resolve: {
      alias: {
        '@rspress/core/theme': path.join(__dirname, 'theme/index.tsx'),
      },
    },
  },
  lang: 'x-default',
  locales: [
    {
      // Rspress requires the default lang to be listed for root-page SSG.
      // The default theme switchers hide this placeholder in global.css.
      lang: 'x-default',
      label: 'Language',
      title: 'IsolaRail',
      description: 'Bilingual product and engineering documentation for IsolaRail.',
    },
    {
      lang: 'zh',
      label: '中文',
      title: 'IsolaRail',
      description: 'IsolaRail 产品与工程文档。',
    },
    {
      lang: 'en',
      label: 'English',
      title: 'IsolaRail',
      description: 'Product and engineering documentation for IsolaRail.',
    },
  ],
  i18nSource: (source: Record<string, Record<string, string>>) =>
    Object.fromEntries(
      Object.entries(source).map(([key, value]) => [
        key,
        {
          ...value,
          'x-default': value.en ?? Object.values(value)[0] ?? key,
        },
      ]),
    ),
  title: 'IsolaRail',
  description: 'Bilingual product and engineering documentation for IsolaRail.',
  icon: '/favicon.ico',
  logoText: 'IsolaRail',
  outDir: 'doc_build',
  globalStyles: path.join(__dirname, 'styles/global.css'),
  head: [
    ['link', { rel: 'icon', href: withBase('favicon.ico') }],
    ['link', { rel: 'icon', href: withBase('favicon-light.ico'), media: '(prefers-color-scheme: light)' }],
    ['link', { rel: 'icon', href: withBase('favicon-dark.ico'), media: '(prefers-color-scheme: dark)' }],
    ['link', { rel: 'icon', type: 'image/png', sizes: '32x32', href: withBase('favicon-32x32.png') }],
    ['link', { rel: 'icon', type: 'image/png', sizes: '16x16', href: withBase('favicon-16x16.png') }],
    [
      'link',
      {
        rel: 'icon',
        type: 'image/png',
        sizes: '32x32',
        href: withBase('favicon-light-32x32.png'),
        media: '(prefers-color-scheme: light)',
      },
    ],
    [
      'link',
      {
        rel: 'icon',
        type: 'image/png',
        sizes: '32x32',
        href: withBase('favicon-dark-32x32.png'),
        media: '(prefers-color-scheme: dark)',
      },
    ],
    ['link', { rel: 'apple-touch-icon', sizes: '180x180', href: withBase('apple-touch-icon.png') }],
    ['link', { rel: 'manifest', href: withBase('site.webmanifest') }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'IsolaRail' }],
    ['meta', { property: 'og:description', content: 'Bilingual product and engineering documentation for IsolaRail.' }],
    ['meta', { property: 'og:image', content: socialPreviewUrl }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:title', content: 'IsolaRail' }],
    ['meta', { name: 'twitter:description', content: 'Bilingual product and engineering documentation for IsolaRail.' }],
    ['meta', { name: 'twitter:image', content: socialPreviewUrl }],
    ['meta', { name: 'theme-color', content: '#0f1720' }],
  ],
  route: {
    cleanUrls: true,
  },
  themeConfig: {
    nav: [
      { text: '中文', link: '/zh/' },
      { text: 'English', link: '/en/' },
    ],
    sidebar: {
      '/zh/': zhSidebar,
      '/en/': enSidebar,
    },
    locales: [
      {
        lang: 'x-default',
        label: 'Language',
        nav: [
          { text: '中文', link: '/zh/' },
          { text: 'English', link: '/en/' },
        ],
        sidebar: {},
      },
      {
        lang: 'zh',
        label: '中文',
        nav: zhNav,
        sidebar: {
          '/zh/': zhSidebar,
        },
      },
      {
        lang: 'en',
        label: 'English',
        nav: enNav,
        sidebar: {
          '/en/': enSidebar,
        },
      },
    ],
    localeRedirect: 'never',
    darkMode: true,
    enableContentAnimation: true,
    enableAppearanceAnimation: true,
    enableScrollToTop: true,
    search: true,
    footer: {
      message: 'IsolaRail documentation. Hardware evidence and firmware contracts live in the repository docs.',
    },
    socialLinks: [
      {
        icon: 'github',
        mode: 'link',
        content: 'https://github.com/IvanLi-CN/isolarail',
      },
    ],
  },
});
