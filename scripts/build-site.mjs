#!/usr/bin/env node
// Builds the public website (docs/, served by GitHub Pages) from:
//   site/template.html      the one page shell (head, nav, footer)
//   site/i18n/<lang>.json   every text, one file per language (es is the root, the rest in subfolders)
//   site/site.css           the brand layer on top of the kit in docs/assets
//   CHANGELOG.md            the changelog page
//
//   npm run site            rebuild every page, sitemap.xml, robots.txt, llms.txt, 404.html, icons
//   npm run site -- --og    also render the share images (docs/share-<lang>.jpg) with headless Chrome
//
// Adding a language: copy site/i18n/en.json to site/i18n/<code>.json, translate it, set "dir",
// add the code to LANG_ORDER below and run npm run site. A page missing from a language is simply not built
// there; hreflang, the sitemap and the language switcher follow automatically.
import { readFileSync, writeFileSync, mkdirSync, existsSync, copyFileSync, readdirSync, rmSync, statSync, mkdtempSync } from 'node:fs'
import { join, dirname, posix } from 'node:path'
import { fileURLToPath } from 'node:url'
import { tmpdir } from 'node:os'
import { spawn } from 'node:child_process'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const DOCS = join(ROOT, 'docs')
const SITE_DIR = join(ROOT, 'site')
const SITE = 'https://pablonolascoo.github.io/couchdesk/'
const BASE_PATH = '/couchdesk/'
const REPO = 'https://github.com/pablonolascoo/couchdesk'
const LATEST = `${REPO}/releases/latest`
const LANG_ORDER = ['es', 'en', 'pt-br', 'fr', 'de', 'it']
const PAGE_ORDER = ['home', 'download', 'changelog', 'vs-xbox', 'vs-bigpicture', 'vs-playnite', 'guide-boot', 'guide-remoteplay', 'guide-controller', 'guide-emulators']
const X_DEFAULT = 'en'
const TODAY = new Date().toISOString().slice(0, 10)

const read = (p) => readFileSync(p, 'utf8').replace(/^\uFEFF/, '')
const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
const write = (p, s) => { mkdirSync(dirname(p), { recursive: true }); writeFileSync(p, s) }

// ------------------------------------------------------------------ data
const langs = LANG_ORDER.filter((c) => existsSync(join(SITE_DIR, 'i18n', `${c}.json`)))
  .map((c) => ({ code: c, ...JSON.parse(read(join(SITE_DIR, 'i18n', `${c}.json`))) }))
const byCode = Object.fromEntries(langs.map((l) => [l.code, l]))
const pkg = JSON.parse(read(join(ROOT, 'package.json')))
const changelogMd = read(join(ROOT, 'CHANGELOG.md'))
const released = changelogMd.match(/^## \[(\d+\.\d+\.\d+)\]\s*[—–-]\s*([^—–\n]+?)\s*[—–-]\s*(\d{4}-\d{2}-\d{2})/m)
const VERSION = released ? released[1] : pkg.version
const CODENAME = released ? released[2].trim() : ''
const RELEASE_DATE = released ? released[3] : TODAY

// page path (inside the site, no leading slash): "" | "en/" | "en/download/"
const pagePath = (lang, id) => lang.dir + lang.pages[id].slug
const has = (lang, id) => Boolean(lang.pages[id])
const absUrl = (lang, id) => SITE + pagePath(lang, id)
function rel(fromPath, toPath) {
  const r = posix.relative('/' + fromPath, '/' + toPath)
  return r === '' ? './' : r + '/'
}
const rootRel = (fromPath) => (fromPath === '' ? '' : posix.relative('/' + fromPath, '/') + '/')

// ------------------------------------------------------------------ logo (brand/final/lockup.svg, inline)
const lockup = read(join(ROOT, 'brand', 'final', 'lockup.svg'))
  .replace(/<\?xml[^>]*>/, '')
  .replace(/<svg /, '<svg role="img" aria-hidden="true" focusable="false" ')
  .replace(/ width="[^"]*" height="[^"]*"/, '')
  .replace(/fill="#1F1A17"/gi, 'class="logo__c" fill="currentColor"')
  .replace(/fill="#E8643A"/gi, 'class="logo__dot" fill="#E8643A"')
  .trim()

// ------------------------------------------------------------------ screenshots (docs/img/<lang>/, made by scripts/screenshots.mjs)
function shotFile(lang, name) {
  for (const d of [lang.code, 'en', 'es']) {
    if (existsSync(join(DOCS, 'img', d, `${name}.jpg`))) return `img/${d}/${name}`
  }
  return null
}
function picture(lang, fromPath, name, { cls = 'frame__img', sizes = '(max-width: 639px) 800px, 1600px', eager = false, alt } = {}) {
  const f = shotFile(lang, name)
  if (!f) return ''
  const r = rootRel(fromPath)
  const hasWebp = existsSync(join(DOCS, `${f}.webp`))
  const has800 = existsSync(join(DOCS, `${f}-800.jpg`))
  const srcset = (ext) => (has800 ? `${r}${f}-800.${ext} 800w, ${r}${f}.${ext} 1600w` : `${r}${f}.${ext} 1600w`)
  const altText = esc(alt ?? lang.shots[name] ?? '')
  const load = eager ? 'fetchpriority="high"' : 'loading="lazy" decoding="async"'
  return `<picture>${hasWebp ? `<source type="image/webp" srcset="${srcset('webp')}" sizes="${sizes}">` : ''}<img class="${cls}" src="${r}${f}.jpg" srcset="${srcset('jpg')}" sizes="${sizes}" width="1600" height="900" alt="${altText}" ${load}></picture>`
}

// ------------------------------------------------------------------ text helpers
function fill(lang, fromPath, s) {
  return String(s)
    .replace(/\{\{page:([\w-]+)\}\}/g, (_, id) => {
      const target = has(lang, id) ? lang : byCode.en
      if (!has(target, id)) throw new Error(`Link to missing page ${id} (${lang.code})`)
      return rel(fromPath, pagePath(target, id))
    })
    .replace(/\{\{latest\}\}/g, LATEST)
    .replace(/\{\{gh\}\}/g, REPO)
    .replace(/\{version\}/g, VERSION)
    .replace(/\{codename\}/g, CODENAME)
}
const arrow = '<svg class="m-arrow" viewBox="0 0 12 12" aria-hidden="true"><path class="m-arrow__stem" d="M1 6h8"/><path class="m-arrow__head" d="M4.5 2.5 8 6l-3.5 3.5"/></svg>'
const chev = '<svg class="chev" viewBox="0 0 12 12" aria-hidden="true"><path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>'
const globe = '<svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M1.5 8h13M8 1.5c2 2 2.6 4.2 2.6 6.5S10 12.5 8 14.5M8 1.5C6 3.5 5.4 5.7 5.4 8S6 12.5 8 14.5" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>'

// ------------------------------------------------------------------ nav, switcher, footer
function navItems(lang, fromPath, id) {
  const u = lang.ui
  const homeP = pagePath(lang, 'home')
  const features = id === 'home' ? '#biblioteca' : rel(fromPath, homeP) + '#biblioteca'
  const group = (g) => PAGE_ORDER.filter((p) => has(lang, p) && lang.pages[p].group === g)
  const menu = (g, span) => group(g).map((p) => {
    const pg = lang.pages[p]
    const cur = p === id ? ' aria-current="page"' : ''
    return `            <a class="menu-card span-${span}" href="${rel(fromPath, pagePath(lang, p))}"${cur}>
              <div class="panel">${picture(lang, fromPath, pg.img, { sizes: '400px' }).replace('<img ', '<img style="object-position: 0 0" ')}</div>
              <span class="t-card">${esc(pg.menu)}</span>
              <span class="t-small muted">${esc(pg.menuSub)}</span>
            </a>`
  }).join('\n')
  const item = (label, g, span) => group(g).length ? `      <div class="nav__item">
        <a class="nav__link" href="${rel(fromPath, pagePath(lang, group(g)[0]))}">${esc(label)} ${chev}</a>
        <div class="nav__menu">
          <div class="wrap grid">
${menu(g, span)}
          </div>
        </div>
      </div>` : ''
  const out = [
    `      <div class="nav__item"><a class="nav__link" href="${features}">${esc(u.navFeatures)}</a></div>`,
    item(u.navCompare, 'compare', 4),
    item(u.navGuides, 'guides', 3),
  ]
  const cl = has(lang, 'changelog') ? lang : byCode.es
  out.push(`      <div class="nav__item"><a class="nav__link" href="${rel(fromPath, pagePath(cl, 'changelog'))}"${id === 'changelog' ? ' aria-current="page"' : ''}>${esc(u.navChangelog)}</a></div>`)
  return out.filter(Boolean).join('\n')
}

function langSwitch(lang, fromPath, id) {
  const items = langs.map((l) => {
    const target = has(l, id) ? pagePath(l, id) : pagePath(l, 'home')
    const cur = l.code === lang.code ? ' aria-current="true"' : ''
    return `          <li><a href="${rel(fromPath, target)}" hreflang="${l.hreflang}" lang="${l.hreflang}"${cur}>${esc(l.name)} <span>${l.code.toUpperCase()}</span></a></li>`
  }).join('\n')
  return `      <details class="lang">
        <summary aria-label="${esc(lang.ui.language)}: ${esc(lang.name)}">${globe}${lang.code.toUpperCase()}</summary>
        <ul>
${items}
        </ul>
      </details>`
}

function footerCols(lang, fromPath) {
  const u = lang.ui
  const links = (g) => PAGE_ORDER.filter((p) => has(lang, p) && lang.pages[p].group === g)
    .map((p) => `<li><a href="${rel(fromPath, pagePath(lang, p))}">${esc(lang.pages[p].menu)}</a></li>`).join('')
  const langsList = langs.map((l) => `<li><a href="${rel(fromPath, pagePath(l, 'home'))}" hreflang="${l.hreflang}" lang="${l.hreflang}">${esc(l.name)}</a></li>`).join('')
  return `        <div class="span-2 start-5 footer__col"><h3>${esc(u.footerCompare)}</h3><ul>${links('compare')}</ul></div>
        <div class="span-2 footer__col"><h3>${esc(u.footerGuides)}</h3><ul>${links('guides')}</ul></div>
        <div class="span-2 footer__col"><h3>${esc(u.footerProject)}</h3><ul><li><a href="${rel(fromPath, pagePath(lang, 'download'))}">${esc(u.downloadShort)}</a></li><li><a href="${REPO}">GitHub</a></li><li><a href="${REPO}/releases">${esc(u.releases)}</a></li><li><a href="${REPO}/issues">${esc(u.issues)}</a></li><li><a href="${REPO}/blob/main/TRANSLATING.md">${esc(u.translate)}</a></li></ul></div>
        <div class="span-2 footer__col"><h3>${esc(u.language)}</h3><ul>${langsList}</ul></div>`
}

// ------------------------------------------------------------------ structured data
const person = { '@type': 'Person', '@id': SITE + '#author', name: 'Pablo Nolasco', url: 'https://github.com/pablonolascoo', sameAs: ['https://github.com/pablonolascoo'] }
function softwareApp(lang) {
  const h = lang.pages.home
  return {
    '@type': 'SoftwareApplication', '@id': SITE + '#app', name: 'CouchDesk',
    description: h.description, url: absUrl(lang, 'home'),
    operatingSystem: 'Windows 10, Windows 11', applicationCategory: 'GameApplication',
    softwareVersion: VERSION, datePublished: RELEASE_DATE, softwareRequirements: '64-bit Windows 10 or Windows 11',
    downloadUrl: LATEST, installUrl: LATEST, releaseNotes: absUrl(has(lang, 'changelog') ? lang : byCode.en, 'changelog'),
    license: 'https://opensource.org/licenses/MIT', isAccessibleForFree: true,
    image: SITE + 'share-' + lang.code + '.jpg',
    screenshot: shotFile(lang, 'inicio') ? SITE + shotFile(lang, 'inicio') + '.jpg' : undefined,
    inLanguage: langs.map((l) => l.hreflang),
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'EUR' },
    author: { '@id': SITE + '#author' }, publisher: { '@id': SITE + '#author' },
    codeRepository: REPO,
  }
}
function jsonLd(lang, id) {
  const pg = lang.pages[id]
  const url = absUrl(lang, id)
  const graph = [
    { '@type': 'WebSite', '@id': SITE + '#website', url: SITE, name: 'CouchDesk', inLanguage: langs.map((l) => l.hreflang), publisher: { '@id': SITE + '#author' } },
    person,
  ]
  if (id === 'home' || id === 'download') graph.push(softwareApp(lang))
  const page = { '@type': id.startsWith('guide-') ? ['WebPage', 'HowTo'] : 'WebPage', '@id': url + '#page', url, name: pg.title, description: pg.description, inLanguage: lang.hreflang, isPartOf: { '@id': SITE + '#website' }, about: { '@id': SITE + '#app' } }
  if (id.startsWith('guide-')) {
    const steps = (pg.blocks || []).filter((b) => b.steps).flatMap((b) => b.steps)
    page.name = pg.h1
    page.step = steps.map((s, i) => ({ '@type': 'HowToStep', position: i + 1, text: fill(lang, '', s).replace(/<[^>]+>/g, '') }))
  }
  if (id !== 'home') {
    page.breadcrumb = { '@type': 'BreadcrumbList', itemListElement: [
      { '@type': 'ListItem', position: 1, name: lang.ui.home, item: absUrl(lang, 'home') },
      { '@type': 'ListItem', position: 2, name: pg.crumb, item: url },
    ] }
  }
  graph.push(page)
  if (id === 'home' && pg.faq) {
    graph.push({ '@type': 'FAQPage', '@id': url + '#faq', inLanguage: lang.hreflang, mainEntity: pg.faq.map((f) => ({
      '@type': 'Question', name: f.q, acceptedAnswer: { '@type': 'Answer', text: fill(lang, '', f.a).replace(/<[^>]+>/g, '') },
    })) })
  }
  return JSON.stringify({ '@context': 'https://schema.org', '@graph': graph }, null, 1).replace(/</g, '\\u003c')
}

// ------------------------------------------------------------------ page bodies
function homeBody(lang, P) {
  const h = lang.pages.home
  const t = (k) => esc(h[k])
  const pic = (name, o) => picture(lang, P, name, o)
  const dg = (o) => esc(JSON.stringify(o)).replace(/&quot;/g, '"')
  const hub = { layout: 'hub', alt: h.dgHubAlt, size: 'card', center: { label: 'CouchDesk', icon: 'ui:monitor' }, spokes: [{ label: 'Xbox', icon: 'ui:gamepad-2' }, { label: 'PS4 · PS5', icon: 'playstation' }, { label: h.dgPhone, icon: 'ui:smartphone' }, { label: h.dgKeyboard, icon: 'ui:keyboard' }] }
  const lanes = { layout: 'lanes', curve: 'curve', ground: 'none', alt: h.dgLanesAlt, input: { label: h.dgPhone, icon: 'ui:smartphone', meta: h.dgQr }, core: { label: 'CouchDesk', icon: 'ui:monitor' }, lanes: [{ label: h.dgPad, icon: 'ui:gamepad-2', done: true }, { label: h.dgTouchpad, icon: 'ui:mouse-pointer-2', done: true }, { label: h.dgKeyboard, icon: 'ui:keyboard', done: true }], output: { label: 'Windows', icon: 'windows', done: true } }
  const vsLinks = ['vs-xbox', 'vs-bigpicture', 'vs-playnite'].filter((p) => has(lang, p)).map((p) => `<a class="link" href="${rel(P, pagePath(lang, p))}">${esc(lang.pages[p].menu)}</a>`).join('')
  const xboxName = (lang.pages['vs-xbox']?.blocks || []).find((b) => b.table)?.table.head[2] || 'Xbox'
  const diffRows = h.diffRows.map((r) => `<tr><th scope="row">${esc(r[0])}</th>${r.slice(1).map((c, i) => `<td${i === 0 ? ' class="is-us"' : ''}>${esc(c)}</td>`).join('')}</tr>`).join('')
  return `  <section class="hero hero--split" data-enter aria-labelledby="hero-h">
    <div class="wrap"><div class="hero-text grid hero__grid">
      <div class="hero__text span-5">
        <p class="hero__kicker"><span class="dot" aria-hidden="true"></span>${esc(fill(lang, P, h.kicker))}</p>
        <h1 id="hero-h">${t('h1')}</h1>
        <p class="hero__lead">${t('lead')}</p>
        <div class="btn-row"><a class="btn btn--primary" href="${rel(P, pagePath(lang, 'download'))}">${esc(lang.ui.download)}${arrow}</a><a class="btn btn--secondary" href="${REPO}">${t('ghButton')}</a></div>
      </div>
      <div class="hero__media span-6 start-7">
        <div class="frame__fx" data-fx="gradient-mesh" data-fx-seed="9" data-fx-palette='{"bg":"#E6DCCB","surface":"#EFE6D8","ink":"#1F1A17","muted":"#65594F","accent":"#E8643A"}' data-fx-params='{"calm":0.2,"calmX":0.4,"calmY":0.5,"tint":0.5,"relief":0.35}'></div>
        <div class="stage stage--fade-br">
          <div class="at-corner" style="--ax: 48px; --ay: 56px; --ay-sm: 24px">
            ${pic('inicio', { cls: 'shot', sizes: '(max-width: 639px) 620px, 1100px', eager: true })}
          </div>
        </div>
      </div>
    </div></div>
  </section>

  <section class="section" id="biblioteca" aria-labelledby="biblioteca-h">
    <div class="wrap">
      <div class="grid grid--top">
        <h2 id="biblioteca-h" class="span-5">${t('libraryH2')}</h2>
        <div class="span-6 start-7 stack-4">
          <p class="t-lead">${t('libraryP')}</p>
          <p><a class="link" href="${rel(P, pagePath(has(lang, 'vs-playnite') ? lang : byCode.en, 'vs-playnite'))}">${t('libraryLink')}</a></p>
        </div>
      </div>
      <div class="frame frame--photo fill">
        ${pic('biblioteca')}
      </div>
    </div>
  </section>

  <section class="section" id="dentro" style="--h: 2" data-moment="pinned-chapters" aria-labelledby="dentro-h">
    <div class="wrap mo-pc__grid">
      <div class="mo-pc__text">
        <h2 id="dentro-h">${t('insideH2')}</h2>
        <ol class="mo-pc__steps">
          <li class="mo-pc__step">
            <h3>${t('cloudH3')}</h3>
            <p>${t('cloudP')}</p>
            <figure class="mo-pc__visual">${pic('nube', { cls: '', sizes: '(max-width: 639px) 342px, (max-width: 1023px) 724px, 628px' })}</figure>
          </li>
          <li class="mo-pc__step">
            <h3>${t('browserH3')}</h3>
            <p>${t('browserP')}</p>
            <figure class="mo-pc__visual">${pic('navegador', { cls: '', sizes: '(max-width: 639px) 342px, (max-width: 1023px) 724px, 628px' })}</figure>
          </li>
          <li class="mo-pc__step">
            <h3>${t('emuH3')}</h3>
            <p>${t('emuP')}</p>
            <figure class="mo-pc__visual">${pic('emuladores', { cls: '', sizes: '(max-width: 639px) 342px, (max-width: 1023px) 724px, 628px' })}</figure>
          </li>
        </ol>
      </div>
      <div class="mo-pc__visuals frame"></div>
    </div>
  </section>

  <section class="section ground-surface" id="mandos" style="--hm: 2" aria-labelledby="mandos-h">
    <div class="wrap">
      <div class="section-head">
        <h2 id="mandos-h">${t('padsH2')}</h2>
      </div>
      <div class="grid">
        <article class="card card--row span-6 m-nolift">
          <h3>${t('padsCardH3')}</h3>
          <p>${t('padsCardP')}</p>
          <div class="panel panel--flush">
            <div class="dg" data-diagram><script type="application/json">${JSON.stringify(hub)}</script></div>
          </div>
        </article>
        <article class="card card--row span-6 m-nolift">
          <h3>${t('phoneCardH3')}</h3>
          <p>${t('phoneCardP')}</p>
          <div class="panel panel--flush">
            <div class="dg" data-dg-still data-diagram><script type="application/json">${JSON.stringify(lanes)}</script></div>
          </div>
        </article>
      </div>
    </div>
  </section>

  <section class="section" id="salon" aria-labelledby="salon-h">
    <div class="wrap">
      <div class="grid grid--top">
        <div class="span-5 stack-6">
          <h2 id="salon-h">${t('roomH2')}</h2>
          <div class="room-list">
            <div><h3>${t('screensaverH3')}</h3><p>${t('screensaverP')}</p></div>
            <div><h3>${t('ambienceH3')}</h3><p>${t('ambienceP')}</p></div>
            <div><h3>${t('musicH3')}</h3><p>${t('musicP')}</p></div>
            <div><h3>${t('claudeH3')}</h3><p>${t('claudeP')}</p></div>
          </div>
        </div>
        <div class="frame frame--photo span-6 start-7 room-shot">
          ${pic('salvapantallas', { sizes: '(max-width: 639px) 800px, 760px' })}
        </div>
      </div>
    </div>
  </section>

  <section class="section" id="salida" aria-labelledby="salida-h">
    <div class="wrap">
      <div class="section-head" data-moment="liquid-type">
        <h2 id="salida-h" class="mo-lt__type">${t('exitH2')}</h2>
        <p>${t('exitP')}</p>
      </div>
      <div class="grid">
        <div class="span-4 stack-4"><p class="t-stat">Esc</p><p class="t-card">${t('exitKeyH')}</p><p>${t('exitKeyP')}</p></div>
        <div class="span-4 stack-4"><p class="t-stat">${t('exitPadStat')}</p><p class="t-card">${t('exitPadH')}</p><p>${t('exitPadP')}</p></div>
        <div class="span-4 stack-4"><p class="t-stat">${t('exitPhoneStat')}</p><p class="t-card">${t('exitPhoneH')}</p><p>${t('exitPhoneP')}</p></div>
      </div>
    </div>
  </section>

  <section class="bg-dots" id="arranque" aria-labelledby="arranque-h">
    <div class="wrap">
      <div class="grid grid--top">
        <div class="span-5 stack-6">
          <h2 id="arranque-h">${t('bootH2')}</h2>
          <p class="t-lead">${t('bootLead')}</p>
          <div class="stack-4">
            <p>${t('bootP1')}</p>
            <p>${t('bootP2')}</p>
            <p>${t('bootP3')}</p>
            <p><a class="link" href="${rel(P, pagePath(has(lang, 'guide-boot') ? lang : byCode.en, 'guide-boot'))}">${t('bootLink')}</a></p>
          </div>
        </div>
        <div class="frame frame--split frame--lit span-6 start-7">
          <div class="stage stage--fade-br"><div class="at-corner" style="--ax: 56px; --ay: 56px">${pic('ajustes', { cls: 'shot', sizes: '900px' })}</div></div>
        </div>
      </div>
    </div>
  </section>

  <section class="section" id="diferencias" aria-labelledby="diferencias-h">
    <div class="wrap">
      <div class="section-head">
        <h2 id="diferencias-h">${t('diffH2')}</h2>
        <p>${t('diffP')}</p>
      </div>
      <div class="table-wrap"><table class="cmp">
        <thead><tr><th scope="col"><span class="visually-hidden"></span></th><th scope="col">CouchDesk</th><th scope="col">${esc(xboxName)}</th><th scope="col">Steam Big Picture</th><th scope="col">Playnite</th></tr></thead>
        <tbody>${diffRows}</tbody>
      </table></div>
      <div class="btn-row diff-links">${vsLinks}</div>
    </div>
  </section>

  <section class="bg-band bg-bleed" id="preguntas" aria-labelledby="preguntas-h">
    <div class="wrap">
      <div class="grid grid--top">
        <div class="span-5 stack-4">
          <h2 id="preguntas-h">${t('faqH2')}</h2>
        </div>
        <div class="faq span-6 start-7">
${h.faq.map((f) => `          <div class="faq__item"><h3>${esc(f.q)}</h3><p>${fill(lang, P, f.a)}</p></div>`).join('\n')}
        </div>
      </div>
    </div>
  </section>`
}

function blockHtml(lang, P, b, id) {
  const f = (s) => fill(lang, P, s)
  if (b.h2) return `<h2>${esc(b.h2)}</h2>`
  if (b.h3) return `<h3>${esc(b.h3)}</h3>`
  if (b.p) return `<p>${f(b.p)}</p>`
  if (b.note) return `<p class="note">${f(b.note)}</p>`
  if (b.list) return `<ul>${b.list.map((x) => `<li>${f(x)}</li>`).join('')}</ul>`
  if (b.steps) return `<ol>${b.steps.map((x) => `<li>${f(x)}</li>`).join('')}</ol>`
  if (b.table) {
    const [head, ...cols] = b.table.head
    return `<div class="table-wrap"><table class="cmp"><thead><tr><th scope="col">${esc(head)}</th>${cols.map((c) => `<th scope="col">${esc(c)}</th>`).join('')}</tr></thead><tbody>${b.table.rows.map((r) => `<tr><th scope="row">${esc(r[0])}</th>${r.slice(1).map((c, i) => `<td${i === 0 ? ' class="is-us"' : ''}>${esc(c)}</td>`).join('')}</tr>`).join('')}</tbody></table></div><p class="t-small muted">${esc(lang.ui.reviewed)}</p>`
  }
  if (b.sources) return `<h2>${esc(lang.ui.sources)}</h2><ul>${b.sources.map(([label, url]) => `<li><a href="${esc(url)}" rel="noopener">${esc(label)}</a></li>`).join('')}</ul>`
  if (b.cta) return `<div class="btn-row"><a class="btn btn--primary" href="${id === 'download' ? LATEST : rel(P, pagePath(lang, 'download'))}">${esc(lang.ui.download)}${arrow}</a><a class="btn btn--secondary" href="${REPO}">GitHub</a></div>`
  if (b.img) return `<figure>${picture(lang, P, b.img, { cls: '', sizes: '(max-width: 1023px) 100vw, 760px' })}</figure>`
  throw new Error('Unknown block ' + JSON.stringify(b))
}

function crumbs(lang, P, id) {
  return `<nav aria-label="Breadcrumb"><ol class="crumbs"><li><a href="${rel(P, pagePath(lang, 'home'))}">${esc(lang.ui.home)}</a></li><li aria-current="page">${esc(lang.pages[id].crumb)}</li></ol></nav>`
}

function aside(lang, P, id) {
  const pg = lang.pages[id]
  const list = (g) => PAGE_ORDER.filter((p) => has(lang, p) && lang.pages[p].group === g)
    .map((p) => `<li><a href="${rel(P, pagePath(lang, p))}"${p === id ? ' aria-current="page"' : ''}>${esc(lang.pages[p].menu)}</a></li>`).join('')
  return `<aside class="aside span-4 start-9" aria-label="${esc(lang.ui.related)}">
          <h2>${esc(lang.ui.related)}</h2>
          <div><p class="t-label muted">${esc(lang.ui.footerCompare)}</p><ul>${list('compare')}</ul></div>
          <div><p class="t-label muted">${esc(lang.ui.footerGuides)}</p><ul>${list('guides')}</ul></div>
          <a class="btn btn--primary" href="${id === 'download' ? LATEST : rel(P, pagePath(lang, 'download'))}">${esc(lang.ui.download)}</a>
        </aside>`
}

function contentBody(lang, P, id, inner) {
  const pg = lang.pages[id]
  const hero = pg.img ? `<figure>${picture(lang, P, pg.img, { cls: '', sizes: '(max-width: 1023px) 100vw, 760px', eager: true })}</figure>` : ''
  const leadBtn = id === 'download' ? `<div class="btn-row"><a class="btn btn--primary" href="${LATEST}">${esc(lang.ui.download)}${arrow}</a><a class="btn btn--secondary" href="${REPO}">GitHub</a></div>` : ''
  return `  <div class="page-head" data-enter>
    <div class="wrap">
      ${crumbs(lang, P, id)}
      <h1>${esc(fill(lang, P, pg.h1))}</h1>
      <p class="lead">${fill(lang, P, pg.lead)}</p>
      ${leadBtn}
    </div>
  </div>
  <article class="doc">
    <div class="wrap">
      <div class="grid">
        <div class="prose span-7">
          ${hero}
          ${inner}
        </div>
        ${aside(lang, P, id)}
      </div>
    </div>
  </article>`
}

// CHANGELOG.md -> HTML (headings, nested lists, bold, links, code)
function mdInline(s) {
  return esc(s)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\[([^\]]+)\]\((https?:[^)]+)\)/g, '<a href="$2" rel="noopener">$1</a>')
}
function changelogHtml(lang) {
  const body = changelogMd.split(/\r?\n/)
  const out = []
  let stack = [] // list depths open
  let inRelease = false
  const closeLists = (to = 0) => { while (stack.length > to) { out.push('</li></ul>'); stack.pop() } }
  for (const line of body) {
    if (/^# /.test(line)) continue
    const h2 = line.match(/^## \[([^\]]+)\](.*)$/)
    if (h2) {
      closeLists(); if (inRelease) out.push('</section>')
      inRelease = true
      const rest = h2[2].replace(/^\s*[—–-]\s*/, '').split(/\s*[—–-]\s*/).filter(Boolean)
      const unreleased = !/^\d/.test(h2[1])
      const title = unreleased ? lang.pages.changelog.unreleased : `${h2[1]}${rest[0] ? ` · ${rest[0]}` : ''}`
      out.push(`<section class="release"><h2 id="v-${unreleased ? 'next' : h2[1].replace(/\./g, '-')}">${esc(title)}</h2>${rest[1] ? `<p class="release__meta"><time datetime="${rest[1]}">${rest[1]}</time></p>` : ''}`)
      continue
    }
    const h3 = line.match(/^### (.+)$/)
    if (h3) { closeLists(); out.push(`<h3>${mdInline(h3[1])}</h3>`); continue }
    const li = line.match(/^(\s*)- (.+)$/)
    if (li) {
      const depth = Math.floor(li[1].length / 2) + 1
      if (depth > stack.length) { while (stack.length < depth) { out.push('<ul><li>'); stack.push(depth) } }
      else { closeLists(depth); out.push('</li><li>') }
      out.push(mdInline(li[2]))
      continue
    }
    if (line.trim() === '') continue
    if (!inRelease) continue // the intro line is replaced by the page lead
    closeLists(); out.push(`<p>${mdInline(line)}</p>`)
  }
  closeLists(); if (inRelease) out.push('</section>')
  return out.join('\n').replace(/<ul><li><\/li><li>/g, '<ul><li>')
}

// ------------------------------------------------------------------ render one page
const template = read(join(SITE_DIR, 'template.html'))
const HOME_CSS = `<link rel="stylesheet" href="{{root}}assets/backgrounds/backgrounds.css">
<link rel="stylesheet" href="{{root}}assets/icons/icon.css">
<link rel="stylesheet" href="{{root}}assets/diagrams/diagram.css">`
const HOME_HEAD = `<script src="{{root}}assets/backgrounds/backgrounds.js"></script>
<script>document.documentElement.classList.add('mo-js');setTimeout(()=>document.documentElement.classList.add('mo-safe'),2500)</script>
<link rel="modulepreload" href="{{root}}assets/moments/moments.js">
<link rel="modulepreload" href="{{root}}assets/moments/lib.js">
<link rel="modulepreload" href="{{root}}assets/moments/pinned-chapters/moment.js">
<link rel="modulepreload" href="{{root}}assets/moments/liquid-type/moment.js">
<style>
  .shot { display: block; width: var(--shot-w, 1100px); height: auto; max-width: none; border-radius: 12px; border: 1px solid rgba(31, 26, 23, .14); box-shadow: 0 24px 48px -28px rgba(31, 26, 23, .45); }
  .hero__media > .frame__fx { position: absolute; inset: 0; z-index: 0; }
  .menu-card .panel { --p-h: 200px; }
  #salida .grid { margin-top: var(--s-8); }
  #mandos .panel { --p-h: 380px; }
  #mandos .panel > .dg { --ink: #1F1A17; --muted: #1F1A17; --card: #EFE6D8; }
  #salon .room-shot { aspect-ratio: 16 / 9; align-self: start; }
  #arranque .shot { --shot-w: 900px; }
  #arranque .frame--split { --ratio: 100%; }
  #biblioteca .frame__img { object-position: 0 0; }
  #salida .section-head { max-width: none; }
  #diferencias .diff-links { margin-top: var(--s-6); gap: var(--s-6); }
  @media (min-width: 1024px) { main > #dentro[data-moment].is-static { min-height: 200vh; min-height: 200svh; } }
  @media (min-width: 1024px) and (max-width: 1279px) {
    #mandos .panel { --p-h: 280px; }
    .hero .btn-row > .btn--secondary { margin-left: calc(-1 * var(--s-2)); }
  }
  @media (min-width: 640px) and (max-width: 1023px) {
    #mandos .panel { --p-h: 420px; }
    #salon .room-shot { margin-top: var(--s-7); }
    #salida .mo-lt__type { font-size: 13vw; }
    #salida .grid > div { grid-column: auto / span 4; }
    main > #dentro[data-moment]:not(.is-live) { min-height: 200vh; min-height: 200svh; }
    #arranque .frame--split { --ratio: 56%; }
    #dentro .mo-pc__step { grid-template-columns: minmax(0, 1fr); }
    #dentro .mo-pc__step .mo-pc__visual { grid-column: 1; grid-row: auto; margin-top: var(--s-4); }
    #dentro .mo-pc__visual img { max-height: 40svh; }
  }
  @media (min-width: 800px) and (max-width: 1023px) { #salida .mo-lt__type { font-size: 18vw; } #preguntas .faq__item { padding-block: var(--s-5); } }
  @media (max-width: 639px) {
    #mandos .panel { --p-h: 260px; }
    #salon .room-shot { margin-top: var(--s-6); }
    main > #dentro[data-moment]:not(.is-live) { min-height: 200vh; min-height: 200lvh; }
    #salida .grid { margin-top: var(--s-4); }
    #arranque .frame--split { --ratio: 50%; }
    .stage > .at-corner .shot { --shot-w: 620px; }
  }
  @media (max-width: 359px) { .btn--xl.btn--wide { padding-inline: var(--s-4); font-size: 16px; } }
</style>`
const HOME_SCRIPTS = `<script type="module" src="{{root}}assets/diagrams/mount.js"></script>`

function ogImageFor(lang) {
  return existsSync(join(DOCS, `share-${lang.code}.jpg`)) ? `${SITE}share-${lang.code}.jpg` : `${SITE}share.jpg`
}

function renderPage(lang, id) {
  const pg = lang.pages[id]
  const P = pagePath(lang, id)
  const R = rootRel(P)
  const others = langs.filter((l) => has(l, id))
  const xdef = has(byCode[X_DEFAULT], id) ? byCode[X_DEFAULT] : others[0]
  const alternates = [
    ...others.map((l) => `<link rel="alternate" hreflang="${l.hreflang}" href="${absUrl(l, id)}">`),
    `<link rel="alternate" hreflang="x-default" href="${absUrl(xdef, id)}">`,
  ].join('\n')
  const ogAlt = others.filter((l) => l !== lang).map((l) => `<meta property="og:locale:alternate" content="${l.ogLocale}">`).join('\n')
  let body
  if (id === 'home') body = homeBody(lang, P)
  else if (id === 'changelog') {
    const note = pg.sourceNote ? `<p class="note">${esc(pg.sourceNote)}</p>` : ''
    const inner = `${note}<div lang="es">${changelogHtml(lang)}</div>`
    body = contentBody(lang, P, id, inner)
  } else body = contentBody(lang, P, id, (pg.blocks || []).map((b) => blockHtml(lang, P, b, id)).join('\n          '))

  const home = lang.pages.home
  const vars = {
    htmlLang: lang.hreflang, title: esc(pg.title), description: esc(pg.description), canonical: absUrl(lang, id),
    alternates, ogType: id === 'home' ? 'website' : 'article', ogLocale: lang.ogLocale, ogLocaleAlternates: ogAlt,
    ogTitle: esc(id === 'home' ? home.ogTitle : pg.title), ogImage: ogImageFor(lang), ogImageAlt: esc(home.shareAlt),
    root: R, jsonld: jsonLd(lang, id), langFile: lang.code,
    headCss: id === 'home' ? HOME_CSS : '', headExtra: id === 'home' ? HOME_HEAD : '', scriptsExtra: id === 'home' ? HOME_SCRIPTS : '',
    homeHref: id === 'home' ? '#top' : rel(P, pagePath(lang, 'home')),
    downloadHref: rel(P, pagePath(lang, 'download')),
    logo: lockup, navItems: navItems(lang, P, id), langSwitch: langSwitch(lang, P, id), footerCols: footerCols(lang, P),
    closeH2: esc(home.closeH2), closeP: esc(home.closeP), body,
  }
  let html = template
  // twice: values may hold {{root}}
  for (let i = 0; i < 2; i++) {
    html = html.replace(/\{\{([\w.]+)\}\}/g, (m, k) => {
      if (k.startsWith('ui.')) return esc(lang.ui[k.slice(3)] ?? m)
      return k in vars ? vars[k] : m
    })
  }
  const left = html.match(/\{\{[\w.:-]+\}\}|\{version\}|\{codename\}/)
  if (left) throw new Error(`Unfilled placeholder ${left[0]} in ${lang.code}/${id}`)
  return html
}

// ------------------------------------------------------------------ 404 (served at any path, so absolute links)
function notFound() {
  const en = byCode.en
  const abs = (l, id) => BASE_PATH + pagePath(l, id)
  const links = langs.map((l) => `<li><a class="btn btn--secondary" href="${abs(l, 'home')}" hreflang="${l.hreflang}" lang="${l.hreflang}">${esc(l.name)}</a></li>`).join('')
  return `<!doctype html>
<html lang="en" data-type="software" data-radius="round" data-ground="sand" data-buttons="arrow" data-motion>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Page not found | CouchDesk</title>
<meta name="description" content="This page doesn't exist. Go to the CouchDesk home page or download CouchDesk for Windows.">
<meta name="robots" content="noindex">
<link rel="icon" href="${BASE_PATH}favicon.ico" sizes="48x48">
<link rel="icon" href="${BASE_PATH}favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="${BASE_PATH}apple-touch-icon.png">
<meta name="theme-color" content="#F3ECE1">
<meta name="site-kind" content="software">
<meta name="page-type" content="software">
<link rel="stylesheet" href="${BASE_PATH}assets/system.css">
<link rel="stylesheet" href="${BASE_PATH}assets/motion/motion.css">
<link rel="stylesheet" href="${BASE_PATH}assets/layout/layout.css">
<link rel="stylesheet" href="${BASE_PATH}assets/site.css">
</head>
<body>
<main id="top" class="notfound">
  <div class="wrap">
    <a class="logo" href="${abs(en, 'home')}" aria-label="CouchDesk">${lockup}</a>
    <h1 style="margin-top: var(--s-8)">Page not found</h1>
    <p>This address doesn't exist on the CouchDesk site. Página no encontrada. Pick your language or download CouchDesk.</p>
    <ul>${links}</ul>
    <div class="btn-row" style="margin-top: var(--s-6)"><a class="btn btn--primary" href="${abs(en, 'download')}">${esc(en.ui.download)}</a></div>
  </div>
</main>
<script type="module" src="${BASE_PATH}assets/motion/motion.js"></script>
</body>
</html>
`
}

// ------------------------------------------------------------------ sitemap, robots, llms
function sitemap() {
  const urls = []
  for (const id of PAGE_ORDER) {
    const ls = langs.filter((l) => has(l, id))
    const xdef = has(byCode[X_DEFAULT], id) ? byCode[X_DEFAULT] : ls[0]
    const alts = [...ls.map((l) => `    <xhtml:link rel="alternate" hreflang="${l.hreflang}" href="${absUrl(l, id)}"/>`), `    <xhtml:link rel="alternate" hreflang="x-default" href="${absUrl(xdef, id)}"/>`].join('\n')
    for (const l of ls) urls.push(`  <url>\n    <loc>${absUrl(l, id)}</loc>\n    <lastmod>${TODAY}</lastmod>\n${alts}\n  </url>`)
  }
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n${urls.join('\n')}\n</urlset>\n`
}
const robots = `# Note: search engines only read /robots.txt at the root of the host (pablonolascoo.github.io),
# so this file under /couchdesk/ is informative. Submit the sitemap in Google Search Console and
# Bing Webmaster Tools, and run \`npm run indexnow\` after publishing.
User-agent: *
Allow: /

Sitemap: ${SITE}sitemap.xml
`
function llms() {
  const en = byCode.en
  const list = (l) => PAGE_ORDER.filter((id) => has(l, id)).map((id) => `- [${l.pages[id].title}](${absUrl(l, id)})`).join('\n')
  return `# CouchDesk

> CouchDesk is a free, open-source (MIT) app for 64-bit Windows 10 and 11. It replaces the Windows desktop with a console-style living-room interface, like a PS5 or Steam Big Picture, run with a gamepad, the keyboard or a phone. Current version: ${VERSION} "${CODENAME}". Built with Tauri 2 and React by Pablo Nolasco. No account, no ads, no telemetry.

## Features

- One game library: Steam, Epic, GOG and Xbox / Game Pass, plus games from other launchers found in the Start menu ("Mark as game"). All installed apps with their real icons.
- Cloud: Xbox Cloud Gaming in full screen and PS5 Remote Play with chiaki-ng.
- Emulators installed and updated with one click, with game, BIOS and save folders per emulator, and rollback to the previous version. It never downloads games, BIOS files or keys.
- A controller-friendly web browser with 24 streaming shortcuts (Netflix, Prime Video, Disney+, HBO Max, DAZN, Movistar Plus+, YouTube, Twitch, Spotify and more) and safe exits (hold Esc, Xbox/PS button, phone Home).
- Phone as gamepad, touchpad or keyboard via QR code, switching mode automatically.
- Xbox, PS4 and PS5 controllers, wired or Bluetooth, several at once; PlayStation touchpad as a mouse; button mapping profiles; on-screen keyboard.
- Music control for Spotify and any player (now playing, play/pause, next, per-app volume), also by voice.
- Claude AI assistant built in: voice, saved conversations, connectors (MCP), extensions, floating window over games (Ctrl+Alt+Space or View+Y).
- Animated PS5-like backgrounds or your own image, video, YouTube link or 3D model; built-in and custom themes with menu music.
- Apple TV-style screensaver with clock, date and weather, low-power mode, can replace the Windows screensaver. Ambient mode (fireplace, waterfall, rain videos).
- Can boot straight into CouchDesk instead of the Windows desktop (per user), with one button to go back.
- File explorer; receive files from a phone or PC by browser upload, FTP or LocalSend; sound, brightness, power and Wi-Fi settings.
- Languages: Spanish, English, Brazilian Portuguese, French, German, Italian, with community translations.
- Works on Windows 10 and 11. Microsoft's Xbox mode needs Windows 11 24H2 or later.

## Links

- [Download (GitHub Releases)](${LATEST})
- [Source code](${REPO})
- [License (MIT)](${REPO}/blob/main/LICENSE)

## Pages (English)

${list(en)}

## Other languages

${langs.filter((l) => l !== en).map((l) => `### ${l.name}\n\n${list(l)}`).join('\n\n')}

## Note

The installer is not signed yet. Windows SmartScreen may warn: click "More info", then "Run anyway".
`
}

// ------------------------------------------------------------------ static files (fonts, css, icons)
function copyStatic() {
  const fonts = [
    ['@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff2', 'instrument-serif-400.woff2'],
    ['@fontsource/instrument-serif/files/instrument-serif-latin-400-italic.woff2', 'instrument-serif-400-italic.woff2'],
    ['@fontsource-variable/geist/files/geist-latin-wght-normal.woff2', 'geist-wght.woff2'],
    ['@fontsource-variable/geist-mono/files/geist-mono-latin-wght-normal.woff2', 'geist-mono-wght.woff2'],
  ]
  mkdirSync(join(DOCS, 'assets', 'fonts'), { recursive: true })
  for (const [src, dst] of fonts) {
    const p = join(ROOT, 'node_modules', src)
    if (existsSync(p)) copyFileSync(p, join(DOCS, 'assets', 'fonts', dst))
    else if (!existsSync(join(DOCS, 'assets', 'fonts', dst))) console.warn(`  ! missing font ${src} (npm install)`)
  }
  copyFileSync(join(SITE_DIR, 'site.css'), join(DOCS, 'assets', 'site.css'))
  const icons = join(ROOT, 'brand', 'web-icons')
  for (const f of ['favicon.ico', 'favicon.svg', 'apple-touch-icon.png', 'icon-192.png', 'icon-512.png', 'maskable-512.png']) {
    if (existsSync(join(icons, f))) copyFileSync(join(icons, f), join(DOCS, f))
  }
  const manifest = JSON.parse(read(join(icons, 'site.webmanifest')))
  manifest.short_name = 'CouchDesk'
  manifest.start_url = './'
  manifest.scope = './'
  manifest.background_color = '#F3ECE1'
  manifest.icons = manifest.icons.map((i) => ({ ...i, src: i.src.replace(/^\//, '') }))
  write(join(DOCS, 'site.webmanifest'), JSON.stringify(manifest, null, 2) + '\n')
}

// ------------------------------------------------------------------ share images (--og), rendered by headless Chrome
async function renderShareImages() {
  const exe = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', 'C:/Program Files/Microsoft/Edge/Application/msedge.exe'].filter(Boolean).find((p) => existsSync(p))
  if (!exe) { console.warn('  ! no Chrome/Edge found, share images skipped'); return }
  const tmp = mkdtempSync(join(tmpdir(), 'couchdesk-og-'))
  const port = 9800 + Math.floor(Math.random() * 100)
  const proc = spawn(exe, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${join(tmp, 'profile')}`, '--no-first-run', '--hide-scrollbars', '--allow-file-access-from-files', 'about:blank'], { stdio: 'ignore' })
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
  let info
  for (let i = 0; i < 50 && !info; i++) { try { info = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json() } catch { await sleep(200) } }
  const ws = new WebSocket(info.webSocketDebuggerUrl)
  await new Promise((r) => { ws.onopen = r })
  let n = 0; const pend = new Map()
  ws.onmessage = (e) => { const m = JSON.parse(e.data); if (m.id && pend.has(m.id)) { pend.get(m.id)(m.result); pend.delete(m.id) } }
  const send = (method, params = {}, sessionId) => new Promise((r) => { const id = ++n; pend.set(id, r); ws.send(JSON.stringify({ id, method, params, sessionId })) })
  const { targetId } = await send('Target.createTarget', { url: 'about:blank' })
  const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true })
  const s = (m, p) => send(m, p, sessionId)
  await s('Emulation.setDeviceMetricsOverride', { width: 1200, height: 630, deviceScaleFactor: 1, mobile: false })
  const fontUrl = (f) => 'file:///' + join(DOCS, 'assets', 'fonts', f).replace(/\\/g, '/')
  for (const lang of langs) {
    const shot = shotFile(lang, 'inicio')
    const shotUrl = shot ? 'file:///' + join(DOCS, `${shot}.jpg`).replace(/\\/g, '/') : ''
    const html = `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face{font-family:IS;src:url("${fontUrl('instrument-serif-400.woff2')}")}
@font-face{font-family:G;src:url("${fontUrl('geist-wght.woff2')}");font-weight:100 900}
*{margin:0;box-sizing:border-box}html,body{width:1200px;height:630px;overflow:hidden}
body{background:#F3ECE1;color:#1F1A17;font-family:G,sans-serif;position:relative}
.glow{position:absolute;right:-180px;top:-160px;width:760px;height:760px;border-radius:50%;background:radial-gradient(closest-side,rgba(232,100,58,.22),rgba(232,100,58,0))}
.l{position:absolute;left:72px;top:64px;width:560px}
.l svg{height:56px;width:auto;display:block}
h1{font-family:IS,serif;font-weight:400;font-size:84px;line-height:.98;letter-spacing:-.015em;margin-top:64px}
p{margin-top:28px;font-size:24px;line-height:1.4;color:#65594F;font-weight:450}
.tag{position:absolute;left:72px;bottom:56px;display:flex;gap:12px}
.tag span{border:1.5px solid rgba(31,26,23,.25);border-radius:99px;padding:10px 18px;font-size:19px;font-weight:500}
.tag span:first-child{background:#E8643A;border-color:#E8643A}
.shot{position:absolute;left:680px;top:120px;width:820px;border-radius:16px;border:1px solid rgba(31,26,23,.14);box-shadow:0 30px 60px -30px rgba(31,26,23,.5)}
</style></head><body><div class="glow"></div>
${shotUrl ? `<img class="shot" src="${shotUrl}">` : ''}
<div class="l">${lockup.replaceAll('class="logo__c" fill="currentColor"', 'fill="#1F1A17"')}<h1>${esc(lang.pages.home.shareLine)}</h1><p>${esc(lang.pages.home.description.split(/\s?:/)[0])}.</p></div>
<div class="tag"><span>Windows 10 · 11</span><span>MIT</span><span>${esc(lang.ui.download)}</span></div>
</body></html>`
    const file = join(tmp, `og-${lang.code}.html`)
    writeFileSync(file, html)
    await s('Page.navigate', { url: 'file:///' + file.replace(/\\/g, '/') })
    await sleep(1200)
    const { data } = await s('Page.captureScreenshot', { format: 'jpeg', quality: 86, clip: { x: 0, y: 0, width: 1200, height: 630, scale: 1 } })
    writeFileSync(join(DOCS, `share-${lang.code}.jpg`), Buffer.from(data, 'base64'))
    console.log(`  share-${lang.code}.jpg`)
  }
  try { await send('Browser.close') } catch { /* closed */ }
  ws.close(); await sleep(400); try { proc.kill() } catch { /* gone */ }
  try { rmSync(tmp, { recursive: true, force: true }) } catch { /* Windows lock */ }
}

// ------------------------------------------------------------------ build
async function main() {
  copyStatic()
  if (process.argv.includes('--og')) await renderShareImages()
  let count = 0
  for (const lang of langs) {
    for (const id of PAGE_ORDER) {
      if (!has(lang, id)) continue
      const out = join(DOCS, pagePath(lang, id), 'index.html')
      write(out, renderPage(lang, id))
      count++
    }
  }
  write(join(DOCS, '404.html'), notFound())
  write(join(DOCS, 'sitemap.xml'), sitemap())
  write(join(DOCS, 'robots.txt'), robots)
  write(join(DOCS, 'llms.txt'), llms())
  console.log(`Site: ${count} pages in ${langs.length} languages (${langs.map((l) => l.code).join(', ')}), version ${VERSION} "${CODENAME}".`)
  const problems = checkSite()
  if (problems.length) { console.error(`Checks: ${problems.length} problem(s)\n  ` + problems.join('\n  ')); process.exit(1) }
  console.log('Checks: links, assets, titles/descriptions, hreflang, canonical, JSON-LD and sitemap OK.')
}

// ------------------------------------------------------------------ checks (run on every build)
function checkSite() {
  const problems = []
  const walk = (d) => readdirSync(d).flatMap((f) => { const p = join(d, f); return statSync(p).isDirectory() ? (f.startsWith('.') ? [] : walk(p)) : [p] })
  const files = walk(DOCS)
  const htmlFiles = files.filter((f) => f.endsWith('.html'))
  const urlOf = (f) => SITE + posix.relative(DOCS.split('\\').join('/'), dirname(f).split('\\').join('/')).replace(/^\.?$/, '').replace(/([^/])$/, '$1/')
  const hreflangMap = new Map() // url -> {lang: href}
  for (const f of htmlFiles) {
    const html = read(f)
    const name = posix.relative(DOCS.split('\\').join('/'), f.split('\\').join('/'))
    const is404 = name === '404.html'
    // links and assets
    for (const m of html.matchAll(/\s(?:href|src|srcset)="([^"]+)"/g)) {
      for (const part of m[1].split(',').map((x) => x.trim().split(/\s+/)[0])) {
        if (!part || /^(https?:|mailto:|data:|#)/.test(part)) continue
        let target
        if (part.startsWith('/')) {
          if (!part.startsWith(BASE_PATH)) { problems.push(`${name}: link outside the site ${part}`); continue }
          target = join(DOCS, part.slice(BASE_PATH.length))
        } else target = join(dirname(f), part.split(/[?#]/)[0])
        if (existsSync(target) && statSync(target).isDirectory()) target = join(target, 'index.html')
        if (!existsSync(target)) problems.push(`${name}: broken link ${part}`)
      }
    }
    for (const m of html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)) {
      try { JSON.parse(m[1]) } catch (e) { problems.push(`${name}: JSON-LD does not parse (${e.message})`) }
    }
    const title = html.match(/<title>([^<]*)<\/title>/)?.[1].replace(/&amp;/g, '&') ?? ''
    const desc = html.match(/<meta name="description" content="([^"]*)"/)?.[1].replace(/&amp;/g, '&').replace(/&quot;/g, '"') ?? ''
    if (!title || [...title].length > 60) problems.push(`${name}: title ${[...title].length} chars (max 60): ${title}`)
    if (!desc || [...desc].length > 155) problems.push(`${name}: description ${[...desc].length} chars (max 155)`)
    if ((html.match(/<h1[\s>]/g) || []).length !== 1) problems.push(`${name}: needs exactly one h1`)
    if (is404) continue
    const canonical = html.match(/<link rel="canonical" href="([^"]+)"/)?.[1]
    if (canonical !== urlOf(f)) problems.push(`${name}: canonical ${canonical} != ${urlOf(f)}`)
    const alts = Object.fromEntries([...html.matchAll(/<link rel="alternate" hreflang="([^"]+)" href="([^"]+)">/g)].map((m) => [m[1], m[2]]))
    if (!alts['x-default']) problems.push(`${name}: no x-default`)
    if (!Object.values(alts).includes(canonical)) problems.push(`${name}: hreflang list misses itself`)
    hreflangMap.set(canonical, alts)
  }
  // reciprocity: every alternate points back
  for (const [url, alts] of hreflangMap) {
    for (const [lang, href] of Object.entries(alts)) {
      if (lang === 'x-default') continue
      const back = hreflangMap.get(href)
      if (!back) problems.push(`${url}: hreflang ${lang} -> ${href} is not a built page`)
      else if (!Object.values(back).includes(url)) problems.push(`${url}: ${href} does not link back`)
    }
  }
  // sitemap lists exactly the built pages
  const sm = read(join(DOCS, 'sitemap.xml'))
  const locs = new Set([...sm.matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => m[1]))
  for (const url of hreflangMap.keys()) if (!locs.has(url)) problems.push(`sitemap misses ${url}`)
  for (const url of locs) if (!hreflangMap.has(url)) problems.push(`sitemap lists unknown ${url}`)
  return problems
}
main().catch((e) => { console.error(e); process.exit(1) })
