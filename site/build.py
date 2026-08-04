#!/usr/bin/env python3
"""HarnessXML.com static site generator.

Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

Dependency-free ON PURPOSE. This site is the canonical home of a specification
that promises its released versions stay reachable at a permanent URL. A build
that needs `pip install` is a build that stops working the year a transitive
dependency is yanked. Standard library only, no network access at build time,
so `python3 site/build.py` produces a byte-identical site in 2036.

Usage:
    python3 site/build.py                 build into site/public/
    python3 site/build.py --check         build, then verify every internal link
    python3 site/build.py --out DIR       build somewhere else
    python3 site/build.py --serve [PORT]  build and serve locally (default 8000)
"""

from __future__ import annotations

import argparse
import html
import json
import os
import re
import shutil
import sys
from dataclasses import dataclass, field
from pathlib import Path

# --------------------------------------------------------------------------
# Site configuration
# --------------------------------------------------------------------------

SITE_URL = "https://harnessxml.com"
SITE_NAME = "HarnessXML"
SITE_TAGLINE = "The Open Specification for Executable Intelligent System Workflows"
STEWARD = "VisML"
STEWARD_URL = "https://visml.com"
REPO_URL = "https://gitlab.com/visml/harnessxml"
# GitLab puts project routes under /-/ ; GitHub does not. Getting this wrong
# gives every page a 404 "Improve this page" link, which is worse than none.
#   GitLab: /-/edit/main/<path>      GitHub: /edit/main/<path>
REPO_EDIT_PATH = "/-/edit/main/"
CURRENT_SPEC = "v1.0"
SPEC_VERSIONS = ["v1.0"]  # newest last; every released version stays forever

ROOT = Path(__file__).resolve().parent.parent
CONTENT = ROOT / "site" / "content"
SPEC_DIR = ROOT / "spec"
ASSETS = ROOT / "site" / "assets"

# Ordered top-level navigation groups. A page declares `section:` in its front
# matter; anything whose section is not listed here lands in "More", which is a
# deliberate smell — it means someone added a page and forgot the nav.
SECTIONS = [
    ("introduction", "Introduction"),
    ("specification", "Specification"),
    ("language", "Language"),
    ("execution", "Execution Model"),
    ("implementing", "Implementing"),
    ("examples", "Examples"),
    ("project", "Project"),
    ("reference", "Reference"),
]

# Repository documents published directly as site pages, so there is exactly one
# copy of each policy.  filename -> (slug, title, section, order, description)
ROOT_DOCS = {
    "GOVERNANCE.md": (
        "governance", "Governance", "project", 10,
        "Who decides what, how a change is proposed, and what guarantees an "
        "implementer can rely on — including the conflict of interest VisML has.",
    ),
    "CONTRIBUTING.md": (
        "contributing", "Contributing", "project", 20,
        "How to report an ambiguity, propose a change, or contribute a "
        "conformance test to HarnessXML.",
    ),
    "CODE_OF_CONDUCT.md": (
        "code-of-conduct", "Code of Conduct", "project", 30,
        "Argue about the specification as hard as you like. Never about the person.",
    ),
}


# --------------------------------------------------------------------------
# Markdown — a deliberate subset
# --------------------------------------------------------------------------

class Markdown:
    """A small, predictable Markdown renderer.

    Supports what technical specification prose actually needs: headings with
    stable anchors, paragraphs, fenced code, lists (nested), tables,
    blockquotes, horizontal rules, and the inline set. It does NOT support
    reference links, footnotes, or raw inline HTML beyond passthrough blocks —
    unsupported syntax renders literally rather than silently vanishing, which
    is the failure mode that matters when the text is normative.
    """

    def __init__(self) -> None:
        self.headings: list[tuple[int, str, str]] = []
        self._slugs: dict[str, int] = {}

    # -- inline ----------------------------------------------------------
    def inline(self, text: str) -> str:
        # Protect code spans from every other rule, including escaping.
        spans: list[str] = []

        def stash(m: re.Match) -> str:
            spans.append(html.escape(m.group(1)))
            return f"\x00{len(spans) - 1}\x00"

        text = re.sub(r"`([^`]+)`", stash, text)
        text = html.escape(text, quote=False)

        # images before links — the syntaxes overlap
        text = re.sub(
            r"!\[([^\]]*)\]\(([^)\s]+)(?:\s+\"([^\"]*)\")?\)",
            lambda m: '<img src="%s" alt="%s"%s loading="lazy">'
            % (m.group(2), m.group(1), f' title="{m.group(3)}"' if m.group(3) else ""),
            text,
        )
        text = re.sub(
            r"\[([^\]]+)\]\(([^)\s]+)(?:\s+\"([^\"]*)\")?\)",
            lambda m: '<a href="%s"%s%s>%s</a>'
            % (
                m.group(2),
                f' title="{m.group(3)}"' if m.group(3) else "",
                ' rel="noopener" target="_blank"' if m.group(2).startswith("http") else "",
                m.group(1),
            ),
            text,
        )
        text = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)
        text = re.sub(r"(?<![\w*])\*([^*\n]+)\*(?![\w*])", r"<em>\1</em>", text)
        text = re.sub(r"(?<!\w)_([^_\n]+)_(?!\w)", r"<em>\1</em>", text)
        text = re.sub(r"~~([^~]+)~~", r"<del>\1</del>", text)

        # bare autolink
        text = re.sub(
            r"(?<![\"'=>\w])(https?://[^\s<>\"')]+)",
            r'<a href="\1" rel="noopener" target="_blank">\1</a>',
            text,
        )

        for i, span in enumerate(spans):
            text = text.replace(f"\x00{i}\x00", f"<code>{span}</code>")
        return text

    def slug(self, text: str) -> str:
        s = re.sub(r"<[^>]+>", "", text)
        s = re.sub(r"[^\w\s-]", "", s.lower()).strip()
        s = re.sub(r"[\s_]+", "-", s)
        s = re.sub(r"-+", "-", s) or "section"
        # Anchors are permanent URLs into a normative document; a duplicate
        # heading must not silently steal an existing anchor.
        if s in self._slugs:
            self._slugs[s] += 1
            s = f"{s}-{self._slugs[s]}"
        else:
            self._slugs[s] = 0
        return s

    # -- block -----------------------------------------------------------
    def render(self, src: str) -> str:
        lines = src.replace("\r\n", "\n").split("\n")
        out: list[str] = []
        i = 0
        n = len(lines)

        while i < n:
            line = lines[i]

            # fenced code
            m = re.match(r"^(```+|~~~+)\s*([\w+-]*)\s*$", line)
            if m:
                fence, lang = m.group(1), m.group(2)
                i += 1
                buf: list[str] = []
                while i < n and not re.match(rf"^{re.escape(fence[0])}{{{len(fence)},}}\s*$", lines[i]):
                    buf.append(lines[i])
                    i += 1
                i += 1
                code = html.escape("\n".join(buf))
                cls = f' class="language-{lang}"' if lang else ""
                label = f'<span class="code-lang">{html.escape(lang)}</span>' if lang else ""
                out.append(
                    f'<div class="code-block">{label}<pre><code{cls}>{code}</code></pre></div>'
                )
                continue

            # raw HTML passthrough block
            if line.startswith("<") and re.match(r"^<(div|figure|section|aside|table|svg|details)\b", line):
                buf = [line]
                i += 1
                while i < n and lines[i].strip() != "":
                    buf.append(lines[i])
                    i += 1
                out.append("\n".join(buf))
                continue

            # heading
            m = re.match(r"^(#{1,6})\s+(.*)$", line)
            if m:
                level = len(m.group(1))
                text = self.inline(m.group(2).strip())
                anchor = self.slug(m.group(2).strip())
                self.headings.append((level, text, anchor))
                out.append(
                    f'<h{level} id="{anchor}">{text}'
                    f'<a class="anchor" href="#{anchor}" aria-label="Permalink to this section">#</a>'
                    f"</h{level}>"
                )
                i += 1
                continue

            # horizontal rule
            if re.match(r"^\s*([-*_])\s*(\1\s*){2,}$", line):
                out.append("<hr>")
                i += 1
                continue

            # table
            if "|" in line and i + 1 < n and re.match(r"^\s*\|?[\s:|-]+\|[\s:|-]*$", lines[i + 1]):
                header = self._row(line)
                aligns = self._aligns(lines[i + 1])
                i += 2
                body: list[list[str]] = []
                while i < n and "|" in lines[i] and lines[i].strip():
                    body.append(self._row(lines[i]))
                    i += 1
                out.append(self._table(header, aligns, body))
                continue

            # blockquote
            if line.startswith(">"):
                buf = []
                while i < n and lines[i].startswith(">"):
                    buf.append(re.sub(r"^>\s?", "", lines[i]))
                    i += 1
                inner = Markdown().render("\n".join(buf))
                cls = "note"
                stripped = "\n".join(buf).lstrip()
                for marker, kind in (("⚠", "warning"), ("**Note", "note"), ("**Warning", "warning")):
                    if stripped.startswith(marker):
                        cls = kind
                out.append(f'<blockquote class="callout {cls}">{inner}</blockquote>')
                continue

            # list
            if re.match(r"^\s*([-*+]|\d+\.)\s+", line):
                block, i = self._collect_list(lines, i)
                out.append(block)
                continue

            # blank
            if not line.strip():
                i += 1
                continue

            # paragraph
            buf = []
            while i < n and lines[i].strip() and not re.match(
                r"^(#{1,6}\s|```|~~~|>|\s*([-*+]|\d+\.)\s+|\s*([-*_])\s*(\3\s*){2,}$)", lines[i]
            ):
                buf.append(lines[i])
                i += 1
            if buf:
                out.append(f"<p>{self.inline(' '.join(x.strip() for x in buf))}</p>")

        return "\n".join(out)

    def _row(self, line: str) -> list[str]:
        cells = line.strip().strip("|").split("|")
        return [c.strip() for c in cells]

    def _aligns(self, line: str) -> list[str]:
        out = []
        for c in line.strip().strip("|").split("|"):
            c = c.strip()
            if c.startswith(":") and c.endswith(":"):
                out.append("center")
            elif c.endswith(":"):
                out.append("right")
            else:
                out.append("left")
        return out

    def _table(self, header: list[str], aligns: list[str], body: list[list[str]]) -> str:
        def align_of(idx: int) -> str:
            a = aligns[idx] if idx < len(aligns) else "left"
            return f' style="text-align:{a}"' if a != "left" else ""

        th = "".join(f"<th scope=\"col\"{align_of(k)}>{self.inline(c)}</th>" for k, c in enumerate(header))
        rows = []
        for r in body:
            tds = "".join(f"<td{align_of(k)}>{self.inline(c)}</td>" for k, c in enumerate(r))
            rows.append(f"<tr>{tds}</tr>")
        # Wide tables must scroll inside their own box; the page body never
        # scrolls sideways.
        return (
            '<div class="table-wrap"><table><thead><tr>'
            + th
            + "</tr></thead><tbody>"
            + "".join(rows)
            + "</tbody></table></div>"
        )

    def _collect_list(self, lines: list[str], i: int) -> tuple[str, int]:
        def indent_of(s: str) -> int:
            return len(s) - len(s.lstrip())

        base = indent_of(lines[i])
        ordered = bool(re.match(r"^\s*\d+\.\s+", lines[i]))
        items: list[list[str]] = []
        n = len(lines)

        while i < n:
            line = lines[i]
            if not line.strip():
                # a blank line only continues the list if the next line is indented
                if i + 1 < n and lines[i + 1].strip() and indent_of(lines[i + 1]) > base:
                    items[-1].append("")
                    i += 1
                    continue
                break
            ind = indent_of(line)
            if ind < base:
                break
            m = re.match(r"^\s*([-*+]|\d+\.)\s+(.*)$", line)
            if m and ind == base:
                items.append([m.group(2)])
                i += 1
                continue
            if not items:
                break
            items[-1].append(line[base:] if ind > base else line.strip())
            i += 1

        tag = "ol" if ordered else "ul"
        rendered = []
        for item in items:
            head = item[0]
            rest = [x for x in item[1:]]
            inner = self.inline(head)
            if any(x.strip() for x in rest):
                dedented = [re.sub(r"^\s{0,4}", "", x) for x in rest]
                inner += Markdown().render("\n".join(dedented))
            rendered.append(f"<li>{inner}</li>")
        return f"<{tag}>" + "".join(rendered) + f"</{tag}>", i


# --------------------------------------------------------------------------
# Content model
# --------------------------------------------------------------------------

@dataclass
class Page:
    slug: str          # url path without leading/trailing slash, "" for home
    title: str
    description: str
    section: str
    order: int
    body_md: str
    source: Path
    spec_version: str | None = None
    status: str = ""
    html_body: str = ""
    headings: list[tuple[int, str, str]] = field(default_factory=list)

    @property
    def url(self) -> str:
        return "/" if not self.slug else f"/{self.slug}/"

    @property
    def out_path(self) -> str:
        return "index.html" if not self.slug else f"{self.slug}/index.html"


FRONT_RE = re.compile(r"^---\s*\n(.*?)\n---\s*\n", re.S)


def parse_front_matter(text: str) -> tuple[dict[str, str], str]:
    """Minimal `key: value` front matter. No YAML dependency, no nesting."""
    m = FRONT_RE.match(text)
    if not m:
        return {}, text
    meta: dict[str, str] = {}
    for line in m.group(1).split("\n"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            continue
        k, v = line.split(":", 1)
        meta[k.strip()] = v.strip().strip('"').strip("'")
    return meta, text[m.end():]


def load_pages() -> list[Page]:
    pages: list[Page] = []

    for path in sorted(CONTENT.rglob("*.md")):
        meta, body = parse_front_matter(path.read_text(encoding="utf-8"))
        rel = path.relative_to(CONTENT).with_suffix("")
        slug = "" if rel.name == "index" and rel.parent == Path(".") else str(rel).replace(os.sep, "/")
        if slug.endswith("/index"):
            slug = slug[: -len("/index")]
        pages.append(
            Page(
                slug=meta.get("slug", slug),
                title=meta.get("title", path.stem.replace("-", " ").title()),
                description=meta.get("description", ""),
                section=meta.get("section", "reference"),
                order=int(meta.get("order", "999")),
                status=meta.get("status", ""),
                body_md=body,
                source=path,
            )
        )

    # Repository documents are published as-is rather than copied into
    # site/content. A governance policy that exists twice is a governance policy
    # that will eventually say two different things.
    for filename, (slug, title, section, order, desc) in ROOT_DOCS.items():
        path = ROOT / filename
        if not path.exists():
            continue
        meta, body = parse_front_matter(path.read_text(encoding="utf-8"))
        pages.append(
            Page(
                slug=slug, title=title, description=desc,
                section=section, order=order, status="stable",
                body_md=body, source=path,
            )
        )

    # Specification chapters live outside site/content so the spec directory is
    # the thing that gets tagged and frozen per version.
    for version_dir in sorted(SPEC_DIR.glob("v*")):
        if not version_dir.is_dir():
            continue
        version = version_dir.name
        for path in sorted(version_dir.rglob("*.md")):
            meta, body = parse_front_matter(path.read_text(encoding="utf-8"))
            stem = re.sub(r"^\d+[-_]", "", path.stem)
            slug = f"spec/{version}" if stem == "index" else f"spec/{version}/{stem}"
            pages.append(
                Page(
                    slug=slug,
                    title=meta.get("title", stem.replace("-", " ").title()),
                    description=meta.get("description", ""),
                    section=meta.get("section", "specification"),
                    order=int(meta.get("order", "999")),
                    status=meta.get("status", ""),
                    body_md=body,
                    source=path,
                    spec_version=version,
                )
            )

    return pages


# --------------------------------------------------------------------------
# Templates
# --------------------------------------------------------------------------

def nav_html(pages: list[Page], current: Page) -> str:
    """Sidebar grouped by section, ordered by SECTIONS then by `order`."""
    by_section: dict[str, list[Page]] = {}
    for p in pages:
        by_section.setdefault(p.section, []).append(p)
    for v in by_section.values():
        v.sort(key=lambda p: (p.order, p.title))

    known = {k for k, _ in SECTIONS}
    parts = ['<nav class="sidebar-nav" aria-label="Documentation">']
    for key, label in SECTIONS + [("more", "More")]:
        group = by_section.get(key, []) if key != "more" else [
            p for s, ps in by_section.items() if s not in known for p in ps
        ]
        if not group:
            continue
        open_attr = " open" if any(p.slug == current.slug for p in group) or key == "introduction" else ""
        parts.append(f"<details{open_attr}><summary>{html.escape(label)}</summary><ul>")
        for p in group:
            cur = ' aria-current="page"' if p.slug == current.slug else ""
            badge = (
                f'<span class="badge badge-{html.escape(p.status)}">{html.escape(p.status)}</span>'
                if p.status and p.status != "stable"
                else ""
            )
            parts.append(f'<li><a href="{p.url}"{cur}>{html.escape(p.title)}{badge}</a></li>')
        parts.append("</ul></details>")
    parts.append("</nav>")
    return "".join(parts)


def toc_html(page: Page) -> str:
    items = [(lvl, t, a) for (lvl, t, a) in page.headings if 2 <= lvl <= 3]
    if len(items) < 2:
        return ""
    parts = ['<nav class="toc" aria-label="On this page"><h2 class="toc-title">On this page</h2><ul>']
    for lvl, text, anchor in items:
        parts.append(f'<li class="toc-l{lvl}"><a href="#{anchor}">{text}</a></li>')
    parts.append("</ul></nav>")
    return "".join(parts)


def version_selector(page: Page) -> str:
    if not page.spec_version:
        return ""
    opts = []
    for v in reversed(SPEC_VERSIONS):
        sel = " selected" if v == page.spec_version else ""
        latest = " (latest)" if v == CURRENT_SPEC else ""
        opts.append(f'<option value="/spec/{v}/"{sel}>{v}{latest}</option>')
    return (
        '<div class="version-select">'
        '<label for="specver">Specification version</label>'
        f'<select id="specver" onchange="location.href=this.value">{"".join(opts)}</select>'
        "</div>"
    )


def page_html(page: Page, pages: list[Page], prev: Page | None, nxt: Page | None) -> str:
    canonical = SITE_URL + page.url
    desc = page.description or SITE_TAGLINE
    title = page.title if page.slug else f"{SITE_NAME} — {SITE_TAGLINE}"
    full_title = title if not page.slug else f"{page.title} · {SITE_NAME}"

    jsonld = {
        "@context": "https://schema.org",
        "@type": "TechArticle",
        "headline": page.title,
        "description": desc,
        "url": canonical,
        "isPartOf": {"@type": "WebSite", "name": SITE_NAME, "url": SITE_URL},
        "publisher": {"@type": "Organization", "name": STEWARD, "url": STEWARD_URL},
        "license": "https://creativecommons.org/licenses/by/4.0/",
    }

    footer_nav = ""
    if prev or nxt:
        left = (
            f'<a class="pager prev" href="{prev.url}"><span>Previous</span>{html.escape(prev.title)}</a>'
            if prev else "<span></span>"
        )
        right = (
            f'<a class="pager next" href="{nxt.url}"><span>Next</span>{html.escape(nxt.title)}</a>'
            if nxt else "<span></span>"
        )
        footer_nav = f'<nav class="pager-nav" aria-label="Pagination">{left}{right}</nav>'

    status_banner = ""
    if page.status == "draft":
        status_banner = (
            '<div class="banner banner-draft" role="status">'
            "<strong>Draft.</strong> This section is not yet normative and may change "
            "before the version it belongs to is released."
            "</div>"
        )
    elif page.status == "planned":
        status_banner = (
            '<div class="banner banner-planned" role="status">'
            "<strong>Planned.</strong> This section is an outline of intent. It has not "
            "been written yet, and nothing here should be implemented against."
            "</div>"
        )

    edit_path = page.source.relative_to(ROOT)

    return f"""<!doctype html>
<html lang="en" data-theme="auto">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(full_title)}</title>
<meta name="description" content="{html.escape(desc)}">
<link rel="canonical" href="{canonical}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="{SITE_NAME}">
<meta property="og:title" content="{html.escape(title)}">
<meta property="og:description" content="{html.escape(desc)}">
<meta property="og:url" content="{canonical}">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="{html.escape(title)}">
<meta name="twitter:description" content="{html.escape(desc)}">
<link rel="stylesheet" href="/assets/site.css">
<link rel="icon" href="/assets/favicon.svg" type="image/svg+xml">
<script>
/* Set the theme before first paint so the page never flashes the wrong one. */
(function(){{try{{var t=localStorage.getItem('hx-theme');if(t)document.documentElement.setAttribute('data-theme',t);}}catch(e){{}}}})();
</script>
<script type="application/ld+json">{json.dumps(jsonld)}</script>
</head>
<body>
<a class="skip-link" href="#main">Skip to content</a>

<header class="site-header">
  <div class="header-inner">
    <a class="brand" href="/">
      <span class="b-harness">Harness</span><span class="b-xml">XML</span>
    </a>
    <button class="nav-toggle" aria-expanded="false" aria-controls="sidebar" aria-label="Toggle navigation">
      <span aria-hidden="true">☰</span>
    </button>
    <div class="header-search">
      <label class="visually-hidden" for="search">Search the documentation</label>
      <input id="search" type="search" placeholder="Search…" autocomplete="off"
             role="combobox" aria-expanded="false" aria-controls="search-results" aria-autocomplete="list">
      <div id="search-results" class="search-results" role="listbox" aria-label="Search results" hidden></div>
    </div>
    <nav class="header-links" aria-label="Primary">
      <a href="/spec/{CURRENT_SPEC}/">Specification</a>
      <a href="/downloads/">Downloads</a>
      <a href="/governance/">Governance</a>
      <a href="{REPO_URL}" rel="noopener" target="_blank">GitLab</a>
    </nav>
    <button class="theme-toggle" id="theme-toggle" aria-label="Switch colour theme" title="Switch colour theme">
      <span aria-hidden="true">◐</span>
    </button>
  </div>
</header>

<div class="layout">
  <aside class="sidebar" id="sidebar">
    {version_selector(page)}
    {nav_html(pages, page)}
  </aside>

  <main id="main" class="content">
    {status_banner}
    <article>
      {page.html_body}
    </article>
    {footer_nav}
    <footer class="page-footer">
      <p>
        <a href="{REPO_URL}{REPO_EDIT_PATH}{edit_path}" rel="noopener" target="_blank">Improve this page</a>
        · Specification text under
        <a href="https://creativecommons.org/licenses/by/4.0/" rel="noopener" target="_blank">CC BY 4.0</a>
        · Code under
        <a href="https://www.apache.org/licenses/LICENSE-2.0" rel="noopener" target="_blank">Apache 2.0</a>
      </p>
      <p>HarnessXML is created and stewarded by
         <a href="{STEWARD_URL}" rel="noopener" target="_blank">{STEWARD}</a>.
         The specification is open and vendor-neutral — anyone may implement it.</p>
    </footer>
  </main>

  {toc_html(page)}
</div>

<script src="/assets/site.js" defer></script>
</body>
</html>
"""


# --------------------------------------------------------------------------
# Build
# --------------------------------------------------------------------------

def strip_tags(s: str) -> str:
    s = re.sub(r"<(script|style)[^>]*>.*?</\1>", " ", s, flags=re.S)
    s = re.sub(r"<[^>]+>", " ", s)
    return re.sub(r"\s+", " ", html.unescape(s)).strip()


def build(out_dir: Path) -> tuple[list[Page], list[str]]:
    pages = load_pages()

    for p in pages:
        md = Markdown()
        p.html_body = md.render(p.body_md)
        p.headings = md.headings

    # Reading order for prev/next: section order, then page order.
    section_rank = {k: i for i, (k, _) in enumerate(SECTIONS)}
    ordered = sorted(pages, key=lambda p: (section_rank.get(p.section, 99), p.order, p.title))

    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)

    search_index = []
    for idx, page in enumerate(ordered):
        prev = ordered[idx - 1] if idx > 0 else None
        nxt = ordered[idx + 1] if idx + 1 < len(ordered) else None
        target = out_dir / page.out_path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(page_html(page, pages, prev, nxt), encoding="utf-8")

        search_index.append(
            {
                "u": page.url,
                "t": page.title,
                "s": dict(SECTIONS).get(page.section, page.section),
                "d": page.description,
                "b": strip_tags(page.html_body)[:1800],
            }
        )

    (out_dir / "search-index.json").write_text(
        json.dumps(search_index, separators=(",", ":")), encoding="utf-8"
    )

    # assets
    if ASSETS.exists():
        shutil.copytree(ASSETS, out_dir / "assets", dirs_exist_ok=True)

    # downloadable schemas and examples, served from permanent paths
    for version_dir in sorted((ROOT / "schema").glob("v*")):
        shutil.copytree(version_dir, out_dir / "schema" / version_dir.name, dirs_exist_ok=True)
    if (ROOT / "examples").exists():
        shutil.copytree(ROOT / "examples", out_dir / "examples-src", dirs_exist_ok=True)

    # sitemap
    urls = "".join(
        f"<url><loc>{SITE_URL}{p.url}</loc>"
        f"<changefreq>{'monthly' if p.spec_version else 'weekly'}</changefreq>"
        f"<priority>{'1.0' if not p.slug else '0.8' if p.spec_version else '0.6'}</priority></url>"
        for p in ordered
    )
    (out_dir / "sitemap.xml").write_text(
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">' + urls + "</urlset>",
        encoding="utf-8",
    )

    (out_dir / "robots.txt").write_text(
        f"User-agent: *\nAllow: /\n\nSitemap: {SITE_URL}/sitemap.xml\n", encoding="utf-8"
    )

    # 404 — a real one. Returning the homepage for a missing page hides broken
    # links from readers and from search engines.
    notfound = Page(
        slug="404", title="Page not found",
        description="That page does not exist on harnessxml.com.",
        section="reference", order=999,
        body_md=(
            "# Page not found\n\nThat URL does not exist here.\n\n"
            "Released specification versions are permanent — if a link to a "
            "`/spec/` page is broken, it was never valid rather than moved, "
            "because HarnessXML does not move released text.\n\n"
            f"- [Specification {CURRENT_SPEC}](/spec/{CURRENT_SPEC}/)\n"
            "- [Downloads](/downloads/)\n- [Home](/)\n"
        ),
        source=ROOT / "site" / "build.py",
    )
    m404 = Markdown()
    notfound.html_body = m404.render(notfound.body_md)
    notfound.headings = m404.headings
    (out_dir / "404.html").write_text(page_html(notfound, pages, None, None), encoding="utf-8")

    return ordered, sorted({p.url for p in ordered})


def check_links(out_dir: Path, valid_urls: list[str]) -> int:
    """Verify every internal href resolves to something we actually emitted."""
    known = set(valid_urls)
    problems = 0
    for path in out_dir.rglob("*.html"):
        text = path.read_text(encoding="utf-8")
        for href in re.findall(r'href="([^"]+)"', text):
            if href.startswith(("http://", "https://", "mailto:", "#")):
                continue
            target = href.split("#")[0]
            if not target.startswith("/"):
                continue
            if target in known:
                continue
            if (out_dir / target.lstrip("/")).exists():
                continue
            if (out_dir / target.lstrip("/") / "index.html").exists():
                continue
            print(f"  BROKEN {path.relative_to(out_dir)} -> {href}")
            problems += 1
    return problems


def main() -> int:
    ap = argparse.ArgumentParser(description="Build harnessxml.com")
    ap.add_argument("--out", default=str(ROOT / "site" / "public"))
    ap.add_argument("--check", action="store_true", help="verify internal links after building")
    ap.add_argument("--serve", nargs="?", const=8000, type=int, help="serve locally after building")
    args = ap.parse_args()

    out_dir = Path(args.out).resolve()
    pages, urls = build(out_dir)
    print(f"built {len(pages)} pages -> {out_dir}")

    rc = 0
    if args.check:
        problems = check_links(out_dir, urls)
        if problems:
            print(f"FAIL: {problems} broken internal link(s)")
            rc = 1
        else:
            print("links OK")

    if args.serve:
        import functools
        import http.server
        import socketserver

        handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(out_dir))
        with socketserver.TCPServer(("", args.serve), handler) as httpd:
            print(f"serving http://localhost:{args.serve}/  (ctrl-c to stop)")
            try:
                httpd.serve_forever()
            except KeyboardInterrupt:
                pass
    return rc


if __name__ == "__main__":
    sys.exit(main())
