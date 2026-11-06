// One layout for every page of the site: the head search engines and link previews read, the header, the docs'
// sidebar, and the footer. Pages come from build.mjs; links are made absolute with the site's base path.

const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

/** The pet's face, small, for the header: a still of Kuchu's own icon. */
const brand = (base) => `<a class="brand" href="${base}" aria-label="Kuchu, home"><img src="${base}icon-64.png" width="28" height="28" alt="">Kuchu</a>`

export function layout({ page, pages, site }) {
  const { base, origin, store, releases, repo } = site
  const url = origin + base + page.path
  const title = page.path === '' ? page.title : `${page.title} | Kuchu`
  const docs = pages.filter((p) => p.kind === 'doc')
  const at = (p) => (p.path === page.path ? ' aria-current="page"' : '')
  const json = (page.jsonld ?? []).map((j) => `<script type="application/ld+json">${JSON.stringify(j)}</script>`).join('\n    ')

  const head = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>${esc(title)}</title>
    <meta name="description" content="${esc(page.description)}">
    <link rel="canonical" href="${url}">${page.noindex ? '\n    <meta name="robots" content="noindex">' : ''}
    <meta name="theme-color" content="#ffffff">
    <link rel="icon" type="image/png" sizes="32x32" href="${base}icon-32.png">
    <link rel="apple-touch-icon" href="${base}icon-180.png">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Kuchu">
    <meta property="og:title" content="${esc(page.title)}">
    <meta property="og:description" content="${esc(page.description)}">
    <meta property="og:url" content="${url}">
    <meta property="og:image" content="${origin + base}og.png">
    <meta property="og:image:width" content="1200">
    <meta property="og:image:height" content="630">
    <meta property="og:image:alt" content="Kuchu, a small robot desk pet, beside a card showing a Claude Code session at work">
    <meta name="twitter:card" content="summary_large_image">
    ${json}
    <script type="module" src="${page.entry}"></script>
  </head>`

  const header = `
    <a class="skip" href="#main">Skip to the content</a>
    <header class="top">
      <div class="top-inner">
        ${brand(base)}
        <nav class="top-nav" aria-label="Site">
          <a href="${base}docs/"${page.kind === 'doc' ? ' aria-current="true"' : ''}>Docs</a>
          <a href="${base}faq/"${at({ path: 'faq/' })}>FAQ</a>
          <a href="${repo}">GitHub</a>
        </nav>
        <a class="btn${page.kind === 'home' ? '' : ' is-primary'} is-small top-get" href="${base}docs/install/">Get Kuchu</a>
      </div>
    </header>`

  const footer = `
    <footer class="bottom">
      <div class="bottom-inner">
        <nav class="bottom-nav" aria-label="More">
          <a href="${base}docs/">Docs</a>
          <a href="${base}docs/install/">Install</a>
          <a href="${base}docs/troubleshooting/">Troubleshooting</a>
          <a href="${base}faq/">FAQ</a>
          <a href="${base}changelog/">Changelog</a>
          <a href="${base}privacy/">Privacy</a>
          <a href="${repo}">Source code</a>
          <a href="${repo}/issues">Report a problem</a>
        </nav>
        <p>Kuchu ${esc(site.version)} is free and open source, under the MIT licence. © 2026 the Kuchu authors.</p>
        <p>Kuchu is an independent app. It is not made or endorsed by Anthropic. Claude and Claude Code are trademarks of
        Anthropic.</p>
      </div>
    </footer>`

  let main
  if (page.kind === 'home') {
    main = `<main id="main">${page.body}</main>`
  } else if (page.kind === 'doc') {
    const i = docs.findIndex((p) => p.path === page.path)
    const prev = docs[i - 1]
    const next = docs[i + 1]
    main = `<div class="docs">
      <nav class="docs-nav" aria-label="Docs">
        <ul>
          ${docs.map((p) => `<li><a href="${base}${p.path}"${at(p)}>${esc(p.nav)}</a></li>`).join('\n          ')}
        </ul>
      </nav>
      <main id="main" class="article">
        <h1>${esc(page.title)}</h1>
        ${page.body}
        <nav class="pager" aria-label="Next and previous">
          ${prev ? `<a class="pager-prev" href="${base}${prev.path}"><span>Previous</span>${esc(prev.nav)}</a>` : '<span></span>'}
          ${next ? `<a class="pager-next" href="${base}${next.path}"><span>Next</span>${esc(next.nav)}</a>` : ''}
        </nav>
        <p class="edit"><a href="${repo}/blob/main/site/content/${page.source}">Improve this page on GitHub</a></p>
      </main>
    </div>`
  } else {
    main = `<main id="main" class="article is-page">
        <h1>${esc(page.title)}</h1>
        ${page.body}
      </main>`
  }

  return `${head}
  <body class="is-${page.kind}">
    ${header}
    ${main}
    ${footer}
  </body>
</html>
`
}
