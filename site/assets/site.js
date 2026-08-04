/* HarnessXML.com — progressive enhancement only.
   Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0

   Everything here is optional. With JavaScript disabled the site is still a
   complete, navigable, readable specification: the nav is server-rendered,
   the TOC is server-rendered, and search degrades to the browser's own find.
   A standards document that needs a bundle to be readable is not a standard. */

(function () {
  "use strict";

  /* ---------- theme ---------- */

  var root = document.documentElement;
  var toggle = document.getElementById("theme-toggle");

  function systemTheme() {
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark" : "light";
  }

  function currentTheme() {
    var explicit = root.getAttribute("data-theme");
    return explicit === "dark" || explicit === "light" ? explicit : systemTheme();
  }

  if (toggle) {
    toggle.addEventListener("click", function () {
      var next = currentTheme() === "dark" ? "light" : "dark";
      root.setAttribute("data-theme", next);
      try { localStorage.setItem("hx-theme", next); } catch (e) {}
      toggle.setAttribute("aria-label",
        next === "dark" ? "Switch to light theme" : "Switch to dark theme");
    });
  }

  /* ---------- mobile nav ---------- */

  var navToggle = document.querySelector(".nav-toggle");
  var sidebar = document.getElementById("sidebar");
  if (navToggle && sidebar) {
    navToggle.addEventListener("click", function () {
      var open = sidebar.classList.toggle("open");
      navToggle.setAttribute("aria-expanded", open ? "true" : "false");
    });
  }

  /* ---------- TOC scroll spy ---------- */

  var tocLinks = Array.prototype.slice.call(document.querySelectorAll(".toc a"));
  if (tocLinks.length && "IntersectionObserver" in window) {
    var byId = {};
    tocLinks.forEach(function (a) { byId[a.getAttribute("href").slice(1)] = a; });

    var targets = Object.keys(byId)
      .map(function (id) { return document.getElementById(id); })
      .filter(Boolean);

    var visible = {};
    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) { visible[e.target.id] = e.isIntersecting; });
      var firstVisible = targets.filter(function (t) { return visible[t.id]; })[0];
      tocLinks.forEach(function (a) { a.classList.remove("active"); });
      if (firstVisible && byId[firstVisible.id]) byId[firstVisible.id].classList.add("active");
    }, { rootMargin: "-15% 0px -70% 0px" });

    targets.forEach(function (t) { observer.observe(t); });
  }

  /* ---------- search ---------- */

  var input = document.getElementById("search");
  var results = document.getElementById("search-results");
  if (!input || !results) return;

  var index = null;
  var loading = false;
  var activeIndex = -1;

  function loadIndex() {
    if (index || loading) return Promise.resolve(index);
    loading = true;
    return fetch("/search-index.json")
      .then(function (r) { return r.json(); })
      .then(function (data) { index = data; loading = false; return index; })
      .catch(function () { loading = false; return null; });
  }

  input.addEventListener("focus", loadIndex, { once: true });

  function escapeHtml(s) {
    return String(s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  /* Ranking: title hit beats description hit beats body hit, and an
     all-terms match beats a partial one. Small and predictable — a reader
     searching a spec wants the section named after their term first. */
  function score(entry, terms) {
    var t = entry.t.toLowerCase();
    var d = (entry.d || "").toLowerCase();
    var b = (entry.b || "").toLowerCase();
    var total = 0, matched = 0;
    for (var i = 0; i < terms.length; i++) {
      var term = terms[i], s = 0;
      if (t === term) s = 200;
      else if (t.indexOf(term) === 0) s = 120;
      else if (t.indexOf(term) !== -1) s = 80;
      if (d.indexOf(term) !== -1) s += 25;
      var pos = b.indexOf(term);
      if (pos !== -1) s += 12;
      if (s > 0) matched++;
      total += s;
    }
    if (matched < terms.length) total = total / 4;
    return matched ? total : 0;
  }

  function snippet(entry, term) {
    var b = entry.b || "";
    var pos = b.toLowerCase().indexOf(term);
    if (pos === -1) return (entry.d || b).slice(0, 130);
    var start = Math.max(0, pos - 45);
    return (start > 0 ? "…" : "") + b.slice(start, start + 140) + "…";
  }

  function render(list, terms) {
    if (!list.length) {
      results.innerHTML = '<div class="r-empty">No matches.</div>';
      results.hidden = false;
      input.setAttribute("aria-expanded", "true");
      return;
    }
    results.innerHTML = list.map(function (e, i) {
      return '<a href="' + e.u + '" role="option" id="sr-' + i + '"' +
        (i === activeIndex ? ' class="active" aria-selected="true"' : ' aria-selected="false"') + '>' +
        '<div class="r-section">' + escapeHtml(e.s) + "</div>" +
        '<div class="r-title">' + escapeHtml(e.t) + "</div>" +
        '<div class="r-snippet">' + escapeHtml(snippet(e, terms[0])) + "</div>" +
        "</a>";
    }).join("");
    results.hidden = false;
    input.setAttribute("aria-expanded", "true");
  }

  function close() {
    results.hidden = true;
    input.setAttribute("aria-expanded", "false");
    activeIndex = -1;
  }

  var lastList = [];

  function search() {
    var q = input.value.trim().toLowerCase();
    if (q.length < 2) { close(); return; }
    loadIndex().then(function (idx) {
      if (!idx) return;
      var terms = q.split(/\s+/);
      lastList = idx
        .map(function (e) { return { e: e, s: score(e, terms) }; })
        .filter(function (x) { return x.s > 0; })
        .sort(function (a, b) { return b.s - a.s; })
        .slice(0, 12)
        .map(function (x) { return x.e; });
      activeIndex = -1;
      render(lastList, terms);
    });
  }

  var timer = null;
  input.addEventListener("input", function () {
    clearTimeout(timer);
    timer = setTimeout(search, 110);
  });

  input.addEventListener("keydown", function (e) {
    if (e.key === "Escape") { close(); input.blur(); return; }
    if (results.hidden || !lastList.length) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      activeIndex += e.key === "ArrowDown" ? 1 : -1;
      if (activeIndex < 0) activeIndex = lastList.length - 1;
      if (activeIndex >= lastList.length) activeIndex = 0;
      render(lastList, input.value.trim().toLowerCase().split(/\s+/));
      var el = document.getElementById("sr-" + activeIndex);
      if (el) el.scrollIntoView({ block: "nearest" });
      input.setAttribute("aria-activedescendant", "sr-" + activeIndex);
    } else if (e.key === "Enter" && activeIndex >= 0) {
      e.preventDefault();
      location.href = lastList[activeIndex].u;
    }
  });

  document.addEventListener("click", function (e) {
    if (!results.contains(e.target) && e.target !== input) close();
  });

  /* "/" focuses search, the convention every developer already has. */
  document.addEventListener("keydown", function (e) {
    if (e.key === "/" && document.activeElement !== input &&
        !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) {
      e.preventDefault();
      input.focus();
    }
  });
})();
