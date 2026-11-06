// Builds Kuchu's website into dist/: the pages from content/ (Markdown with a little front matter, and the home page's
// HTML) into one layout, then Vite bundles the styles and the live pet, then the files search engines and AI tools
// read (sitemap.xml, robots.txt, llms.txt, llms-full.txt). Served at SITE_ORIGIN + SITE_BASE: GitHub Pages unless
// those say otherwise (a domain of its own is SITE_BASE=/).

import react from '@vitejs/plugin-react'
import { marked } from 'marked'
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { build } from 'vite'
import { layout } from './src/layout.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const repoDir = join(here, '..')
const gen = join(here, '.gen')
const out = join(here, 'dist')

const version = JSON.parse(readFileSync(join(repoDir, 'app/src-tauri/tauri.conf.json'), 'utf8')).version
const repo = 'https://github.com/abhee235/kuchu'
const site = {
  origin: process.env.SITE_ORIGIN ?? 'https://abhee235.github.io',
  base: process.env.SITE_BASE ?? '/kuchu/',
  version,
  repo,
  store: 'https://apps.microsoft.com/detail/9NNK1Q800R84',
  releases: `${repo}/releases/latest`,
}
const vars = {
  BASE: site.base,
  VERSION: version,
  STORE: site.store,
  RELEASES: site.releases,
  REPO: repo,
  DOWNLOAD: `${repo}/releases/latest/download`,
}
/** {{NAME}} in content becomes the site's own value, so links and versions are written once. */
const fill = (text) => text.replace(/\{\{(\w+)\}\}/g, (m, k) => vars[k] ?? m)

// ── Markdown ─────────────────────────────────────────────────────────────────────────────────────────────────────

const slug = (text) =>
  text
    .toLowerCase()
    .replace(/<[^>]+>/g, '')
    .replace(/&[a-z]+;/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')

marked.use({
  gfm: true,
  renderer: {
    // Headings below the title can be linked to.
    heading({ tokens, depth }) {
      const text = this.parser.parseInline(tokens)
      if (depth === 1) return `<h1>${text}</h1>\n`
      const id = slug(text)
      return `<h${depth} id="${id}"><a class="anchor" href="#${id}">${text}</a></h${depth}>\n`
    },
    // A link to /docs/… is the site's own: its base path goes in front. Links off the site open as usual.
    link({ href, title, tokens }) {
      const text = this.parser.parseInline(tokens)
      const to = href.startsWith('/') ? site.base + href.slice(1) : href
      return `<a href="${to}"${title ? ` title="${title}"` : ''}>${text}</a>`
    },
  },
})

/** Front matter: `key: value` lines between two `---` lines at the top. */
function read(file) {
  const raw = fill(readFileSync(file, 'utf8')).replace(/\r\n/g, '\n')
  const m = raw.match(/^---\n([\s\S]*?)\n---\n/)
  const meta = {}
  if (m) for (const line of m[1].split('\n')) {
    const i = line.indexOf(':')
    if (i > 0) meta[line.slice(0, i).trim()] = line.slice(i + 1).trim()
  }
  return { meta, md: m ? raw.slice(m[0].length) : raw }
}

const plain = (html) => html.replace(/<[^>]+>/g, '').replace(/&amp;/g, '&').replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/\s+/g, ' ').trim()

// ── The pages ────────────────────────────────────────────────────────────────────────────────────────────────────

const pages = []

const home = read(join(here, 'content/home.html'))
pages.push({
  kind: 'home',
  path: '',
  title: home.meta.title,
  description: home.meta.description,
  body: home.md,
  md: null,
  jsonld: [
    {
      '@context': 'https://schema.org',
      '@type': 'SoftwareApplication',
      name: 'Kuchu',
      description: home.meta.description,
      applicationCategory: 'DeveloperApplication',
      operatingSystem: 'Windows 10, Windows 11, macOS',
      softwareVersion: version,
      url: site.origin + site.base,
      downloadUrl: site.releases,
      installUrl: site.store,
      image: site.origin + site.base + 'og.png',
      license: 'https://opensource.org/licenses/MIT',
      isAccessibleForFree: true,
      offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
      softwareRequirements: 'Claude Code, installed separately',
    },
    { '@context': 'https://schema.org', '@type': 'WebSite', name: 'Kuchu', url: site.origin + site.base },
  ],
})

const docDir = join(here, 'content/docs')
const docs = readdirSync(docDir)
  .filter((f) => f.endsWith('.md'))
  .map((f) => ({ f, ...read(join(docDir, f)) }))
  .sort((a, b) => Number(a.meta.order) - Number(b.meta.order))
for (const { f, meta, md } of docs) {
  const name = f.replace(/\.md$/, '')
  pages.push({
    kind: 'doc',
    path: name === 'index' ? 'docs/' : `docs/${name}/`,
    source: `docs/${f}`,
    title: meta.title,
    nav: meta.nav ?? meta.title,
    description: meta.description,
    body: marked.parse(md),
    md,
  })
}

// The FAQ: each `## question` and the answer under it, also as FAQPage data for search engines.
const faq = read(join(here, 'content/faq.md'))
const qa = faq.md
  .split(/^## /m)
  .slice(1)
  .map((part) => {
    const [q, ...rest] = part.split('\n')
    return { q: q.trim(), a: plain(marked.parse(rest.join('\n'))) }
  })
pages.push({
  kind: 'page',
  path: 'faq/',
  title: faq.meta.title,
  description: faq.meta.description,
  body: marked.parse(faq.md),
  md: faq.md,
  jsonld: [{ '@context': 'https://schema.org', '@type': 'FAQPage', mainEntity: qa.map(({ q, a }) => ({ '@type': 'Question', name: q, acceptedAnswer: { '@type': 'Answer', text: a } })) }],
})

const changelog = read(join(here, 'content/changelog.md'))
pages.push({ kind: 'page', path: 'changelog/', title: changelog.meta.title, description: changelog.meta.description, body: marked.parse(changelog.md), md: changelog.md })

// The privacy policy is the repository's own PRIVACY.md, so there is one text to keep true; the site adds its own part.
const privacy = readFileSync(join(repoDir, 'PRIVACY.md'), 'utf8').replace(/\r\n/g, '\n').replace(/^# .*\n+/, '')
const privacySite = `\n## This website\n\nThis website has no cookies, no analytics and no trackers, and loads nothing from other sites. GitHub, which hosts
it, keeps its own logs of visits, under GitHub's privacy statement.\n`
pages.push({
  kind: 'page',
  path: 'privacy/',
  title: "Kuchu's privacy policy",
  description: 'Kuchu collects nothing and sends nothing anywhere. What it reads on your computer, what it keeps, and how to remove it.',
  body: marked.parse(privacy + privacySite),
  md: privacy + privacySite,
})

pages.push({
  kind: 'page',
  path: '404.html',
  file: '404.html',
  title: 'Nothing here',
  description: 'This page does not exist on the Kuchu website.',
  body: `<p>Nothing lives at this address. The <a href="${site.base}docs/">docs</a> or the <a href="${site.base}">home page</a> may have what you are looking for.</p>`,
  md: null,
  noindex: true,
})

// ── Write, bundle, finish ────────────────────────────────────────────────────────────────────────────────────────

rmSync(gen, { recursive: true, force: true })
const input = {}
for (const page of pages) {
  const file = join(gen, page.file ?? join(page.path, 'index.html'))
  mkdirSync(dirname(file), { recursive: true })
  page.entry = relative(dirname(file), join(here, 'src/main.ts')).replace(/\\/g, '/')
  writeFileSync(file, layout({ page, pages, site }))
  input[(page.file ?? page.path + 'index').replace(/\.html$/, '') || 'index'] = file
}

await build({
  configFile: false,
  root: gen,
  base: site.base,
  logLevel: 'warn',
  publicDir: join(here, 'public'),
  plugins: [react()],
  // The pet comes from the app's own source; it must use this React, not look for one beside the app.
  resolve: { dedupe: ['react', 'react-dom'] },
  build: { outDir: out, emptyOutDir: true, target: 'es2022', rollupOptions: { input } },
})

// Kuchu's icon, from the app's own, for the header and the browser tab.
const icons = join(repoDir, 'app/src-tauri/icons')
copyFileSync(join(icons, '32x32.png'), join(out, 'icon-32.png'))
copyFileSync(join(icons, '64x64.png'), join(out, 'icon-64.png'))
copyFileSync(join(icons, '128x128@2x.png'), join(out, 'icon-180.png'))

const listed = pages.filter((p) => !p.noindex)
const today = new Date().toISOString().slice(0, 10)
writeFileSync(
  join(out, 'sitemap.xml'),
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${listed
    .map((p) => `  <url><loc>${site.origin + site.base + p.path}</loc><lastmod>${today}</lastmod></url>`)
    .join('\n')}\n</urlset>\n`,
)
writeFileSync(join(out, 'robots.txt'), `User-agent: *\nAllow: /\n\nSitemap: ${site.origin + site.base}sitemap.xml\n`)

// llms.txt: what Kuchu is and where each page is, for AI tools; llms-full.txt: every page's text in one file.
const abs = (p) => site.origin + site.base + p.path
const docPages = pages.filter((p) => p.kind === 'doc')
writeFileSync(
  join(out, 'llms.txt'),
  `# Kuchu\n\n> ${home.meta.description}\n\n${fill(readFileSync(join(here, 'content/llms-intro.md'), 'utf8')).trim()}\n\n## Docs\n\n${docPages
    .map((p) => `- [${p.title}](${abs(p)}): ${p.description}`)
    .join('\n')}\n\n## More\n\n- [FAQ](${abs({ path: 'faq/' })}): ${faq.meta.description}\n- [Changelog](${abs({ path: 'changelog/' })}): ${changelog.meta.description}\n- [Privacy](${abs({ path: 'privacy/' })}): What Kuchu reads, keeps and sends (nothing).\n- [Source code](${repo}): MIT licence.\n- [Microsoft Store](${site.store}): Kuchu for Windows.\n`,
)
writeFileSync(
  join(out, 'llms-full.txt'),
  `# Kuchu\n\n> ${home.meta.description}\n\n` +
    pages
      .filter((p) => p.md)
      .map((p) => `# ${p.title}\n\nURL: ${abs(p)}\n\n${p.md.trim()}\n`)
      .join('\n---\n\n'),
)

rmSync(gen, { recursive: true, force: true })
console.log(`Built ${pages.length} pages into ${relative(process.cwd(), out) || 'dist'}, for ${site.origin}${site.base}`)
