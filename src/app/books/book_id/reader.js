// The full-screen reader. EPUB pages through epub.js; comics are one page per load.
// The place in the book and the text settings are saved on the account, not in this
// browser, so they follow the user everywhere.
(() => {
  const reader = document.getElementById("reader");
  if (!reader) return;

  const put = (url, body, keepalive = false) =>
    fetch(url, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      credentials: "same-origin",
      keepalive,
    }).catch(() => {});

  // ---- Panels: contents and settings, opened by their header buttons. ----

  const panels = [...reader.querySelectorAll("button[aria-controls]")]
    .map((button) => ({ button, panel: document.getElementById(button.getAttribute("aria-controls")) }))
    .filter((p) => p.panel);
  const openPanel = () => panels.find((p) => !p.panel.hidden);
  const close = (p, refocus) => {
    p.panel.hidden = true;
    p.button.setAttribute("aria-expanded", "false");
    if (refocus) p.button.focus();
  };
  const toggle = (p) => {
    if (!p) return;
    const opening = p.panel.hidden;
    panels.forEach((other) => close(other, false));
    if (!opening) return p.button.focus();
    p.panel.hidden = false;
    p.button.setAttribute("aria-expanded", "true");
    const first = p.panel.querySelector("[aria-current='true'], input:checked, a, select, input");
    (first || p.panel).focus();
  };
  const panel = (id) => panels.find((p) => p.panel.id === id);
  panels.forEach((p) => p.button.addEventListener("click", () => toggle(p)));
  document.addEventListener("pointerdown", (event) => {
    const open = openPanel();
    if (open && !open.panel.contains(event.target) && !open.button.contains(event.target)) close(open, false);
  });

  // ---- Keyboard: the same keys whether focus is on the page or in the book. ----

  let turn = { prev() {}, next() {} };
  const onKey = (event) => {
    if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
    const open = openPanel();
    if (event.key === "Escape") {
      if (open) {
        event.preventDefault();
        close(open, true);
      }
      return;
    }
    // Form controls keep their own keys: arrows move a slider or pick a colour; and
    // Space presses a focused button or link. Events from the book's frame come from
    // another window, where `instanceof Element` is false, hence `closest` is checked.
    const within = (selector) => typeof event.target.closest === "function" && event.target.closest(selector);
    if (within("input, select, textarea")) return;
    if (event.key === " " && within("button, a, [role=button]")) return;
    const key = event.key.toLowerCase();
    if (key === "t" && panel("toc")) return event.preventDefault(), toggle(panel("toc"));
    if (key === "s" && panel("settings")) return event.preventDefault(), toggle(panel("settings"));
    if (open && open.panel.contains(document.activeElement)) return;
    if (event.key === "ArrowLeft" || event.key === "PageUp" || (event.key === " " && event.shiftKey)) {
      event.preventDefault();
      turn.prev();
    } else if (event.key === "ArrowRight" || event.key === "PageDown" || event.key === " ") {
      event.preventDefault();
      turn.next();
    }
  };
  document.addEventListener("keydown", onKey);

  // ---- Settings: applied at once, saved to the account shortly after. ----

  const form = document.getElementById("settings");
  let prefs = {
    theme: reader.dataset.readerTheme,
    font: reader.dataset.font,
    size: Number(reader.dataset.size),
    line_height: Number(reader.dataset.lineHeight),
  };
  const onPrefs = [];
  let prefsTimer;
  form?.addEventListener("input", () => {
    const data = new FormData(form);
    prefs = {
      theme: data.get("theme") ?? prefs.theme,
      font: data.get("font") ?? prefs.font,
      size: Number(data.get("size") ?? prefs.size),
      line_height: Number(data.get("line_height") ?? prefs.line_height),
    };
    reader.dataset.readerTheme = prefs.theme;
    const size = document.getElementById("size-value");
    if (size) size.textContent = prefs.size + "%";
    const lineHeight = document.getElementById("line-height-value");
    if (lineHeight) lineHeight.textContent = (prefs.line_height / 100).toFixed(1);
    onPrefs.forEach((apply) => apply());
    clearTimeout(prefsTimer);
    prefsTimer = setTimeout(() => put(reader.dataset.prefsUrl, prefs), 400);
  });
  form?.addEventListener("submit", (event) => event.preventDefault());

  // ---- Comics: each page is its own URL, which the server saves as the place. ----

  if (reader.dataset.format === "CBZ") {
    const go = (id) => {
      const link = document.getElementById(id);
      if (link && link.href !== location.href) location.href = link.href;
    };
    turn = { prev: () => go("prev"), next: () => go("next") };
    return;
  }

  // ---- EPUB ----

  const viewer = document.getElementById("viewer");
  if (!viewer || !window.ePub) return;

  const book = ePub(viewer.dataset.src, { openAs: "epub" });
  const rendition = book.renderTo(viewer, { width: "100%", height: "100%", spread: "auto" });
  turn = { prev: () => rendition.prev(), next: () => rendition.next() };
  document.getElementById("prev").addEventListener("click", turn.prev);
  document.getElementById("next").addEventListener("click", turn.next);

  // The book's pages live in iframes, which see none of this page's fonts.
  const fontSheets = [...document.querySelectorAll("link[rel='stylesheet']")]
    .map((link) => link.href)
    .filter((href) => href.includes("font"));
  const families = {
    literata: "'Literata', Georgia, serif",
    serif: "Georgia, 'Iowan Old Style', 'Times New Roman', serif",
    sans: "'Geist', system-ui, sans-serif",
    atkinson: "'Atkinson Hyperlegible', system-ui, sans-serif",
    dyslexic: "'OpenDyslexic', 'Atkinson Hyperlegible', sans-serif",
  };
  const bookCss = () => {
    const style = getComputedStyle(reader);
    const color = (name) => style.getPropertyValue(name).trim();
    const family = families[prefs.font];
    return `
      html { font-size: ${prefs.size}% !important; }
      html, body { background: ${color("--background")} !important; color: ${color("--foreground")} !important; }
      body * { color: inherit !important; }
      ${prefs.theme === "dark" ? "body * { background-color: transparent !important; }" : ""}
      a, a * { color: ${color("--primary")} !important; }
      body, p, li, blockquote, dd, dt { line-height: ${prefs.line_height / 100} !important; }
      ${family ? `body, body *:not(pre, code, kbd, samp, pre *) { font-family: ${family} !important; }` : ""}
      ::selection { background: color-mix(in srgb, ${color("--primary")} 30%, transparent); }
    `;
  };
  // The place on screen, or null before the first page is laid out.
  const here = () => {
    try {
      const where = rendition.currentLocation();
      return where && where.start ? where : null;
    } catch {
      return null;
    }
  };
  // Injected by hand: epub.js 0.3 does not carry a CSS-string theme into new pages.
  rendition.hooks.content.register((contents) => {
    contents.addStylesheetCss(bookCss(), "reader");
    fontSheets.forEach((href) => contents.addStylesheet(href));
    // After a click in the text, keys go to the book's frame. epub.js's own keydown
    // listener there is passive, so it cannot stop Space from scrolling.
    contents.document.addEventListener("keydown", onKey);
    // Wheel and swipe gestures happen inside the iframe, out of this page's reach.
    let lastWheel = 0;
    contents.document.addEventListener("wheel", (event) => {
      const now = Date.now();
      const start = now - lastWheel > 300;
      lastWheel = now;
      const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
      if (start && Math.abs(delta) > 4) (delta > 0 ? turn.next : turn.prev)();
    }, { passive: true });
    let touchX = null;
    contents.document.addEventListener("touchstart", (event) => { touchX = event.changedTouches[0].clientX; }, { passive: true });
    contents.document.addEventListener("touchend", (event) => {
      const dx = event.changedTouches[0].clientX - touchX;
      if (touchX !== null && Math.abs(dx) > 50) (dx < 0 ? turn.next : turn.prev)();
      touchX = null;
    }, { passive: true });
  });
  onPrefs.push(() => {
    const where = here();
    if (!where) {
      const css = bookCss();
      rendition.getContents().forEach((contents) => contents.addStylesheetCss(css, "reader"));
      return;
    }
    // New text sizes change where pages break. Restyling the open pages in place
    // leaves epub.js measuring the old layout, and the reader drifts pages ahead at
    // every change; laying them out afresh, with the new styles from the hook above,
    // keeps the place.
    rendition.manager.clear();
    rendition.display(where.start.cfi);
  });

  // The place in the book, saved shortly after each turn and when the page is left.
  const progress = document.getElementById("progress");
  const percentLabel = document.getElementById("percent");
  const locationsKey = "reader-locations:" + viewer.dataset.src;
  let measured = false;
  let pending = null;
  let saveTimer;
  const save = (keepalive) => {
    clearTimeout(saveTimer);
    if (pending) put(reader.dataset.positionUrl, pending, keepalive);
    pending = null;
  };
  const percentOf = (location) =>
    location.atEnd ? 100 : Math.round(book.locations.percentageFromCfi(location.start.cfi) * 100);

  // Page numbers: the printed edition's when the book lists them, which stay put
  // whatever the font or window; otherwise the measured locations, about 1600
  // characters each, close to a printed page.
  const pageLabel = document.getElementById("page");
  const cfi = new ePub.CFI();
  let printed = null;
  const pageOf = (location) => {
    if (!printed) {
      const total = book.locations.length();
      const n = location.atEnd ? total : book.locations.locationFromCfi(location.start.cfi) + 1;
      return { n, total };
    }
    // The last printed page that starts at or before the top of the screen.
    let low = 0, high = printed.pages.length - 1, n = null;
    while (low <= high) {
      const mid = (low + high) >> 1;
      if (cfi.compare(printed.pages[mid].cfi, location.start.cfi) <= 0) {
        n = printed.pages[mid].page;
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }
    return n === null ? null : { n, total: printed.total };
  };
  const showPlace = (location) => {
    const percent = percentOf(location);
    progress.value = percent;
    percentLabel.textContent = percent + "%";
    const page = pageOf(location);
    pageLabel.textContent = page ? pageLabel.dataset.template.replace("{n}", page.n).replace("{total}", page.total) : "";
    return percent;
  };

  // epub.js reads a page list only when it links by CFI; most link `file#id`, so
  // each marker is found in its chapter here.
  const printedPages = async () => {
    const items = ((book.pageList && book.pageList.pageList) || []).filter((item) => Number.isFinite(item.page));
    if (!items.length) return null;
    const chapters = new Map();
    for (const item of items) {
      const [href, id] = item.href.split("#");
      if (!chapters.has(href)) chapters.set(href, []);
      chapters.get(href).push({ id, page: item.page });
    }
    const pages = [];
    for (const [href, marks] of chapters) {
      const section = book.spine.get(href);
      if (!section) continue;
      await section.load(book.load.bind(book));
      for (const mark of marks) {
        const element = mark.id ? section.document.getElementById(mark.id) : section.document.body;
        if (element) pages.push({ cfi: section.cfiFromElement(element), page: mark.page });
      }
      section.unload();
    }
    if (!pages.length) return null;
    pages.sort((a, b) => cfi.compare(a.cfi, b.cfi));
    return { pages, total: Math.max(...pages.map((p) => p.page)) };
  };

  rendition.on("relocated", (location) => {
    const percent = measured ? showPlace(location) : null;
    pending = { format: "EPUB", position: location.start.cfi, percent };
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => save(false), 1000);
    markChapter(location.start.href);
  });
  addEventListener("pagehide", () => save(true));
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") save(true);
  });

  // On reopening, light up the line the reader left off at. The saved place is the
  // top of the page they were on. The mark stays for the whole visit, tied to that
  // line: it sits over the book's frame, so the book's own layout is untouched, and
  // is placed again whenever the pages move, showing only while its line is on screen.
  const saved = viewer.dataset.position;
  const band = document.createElement("div");
  band.className = "reader-resume";
  const label = document.createElement("div");
  label.className = "reader-resume-label";
  label.setAttribute("role", "status");
  band.hidden = label.hidden = true;
  // The first visible character at or after a point. A chapter's start often falls
  // in the blank text before its heading, or between two elements, neither of which
  // has a box on screen.
  const firstChar = (range) => {
    const doc = range.startContainer.ownerDocument;
    const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_TEXT);
    let node = range.startContainer;
    let offset = range.startOffset;
    if (node.nodeType !== Node.TEXT_NODE) {
      const child = node.childNodes[offset];
      if (child) {
        walker.currentNode = child;
        node = child.nodeType === Node.TEXT_NODE ? child : walker.nextNode();
      } else {
        walker.currentNode = node;
        node = walker.nextSibling() || walker.nextNode();
      }
      offset = 0;
    } else {
      walker.currentNode = node;
    }
    for (; node; node = walker.nextNode(), offset = 0) {
      const at = node.textContent.slice(offset).search(/\S/);
      if (at < 0) continue;
      const char = doc.createRange();
      char.setStart(node, offset + at);
      char.setEnd(node, offset + at + 1);
      if (char.getClientRects().length) return char;
    }
    return null;
  };
  const placeResume = async () => {
    band.hidden = label.hidden = true;
    const range = saved && rendition.getRange(saved);
    if (!range) return;
    const doc = range.startContainer.ownerDocument;
    await doc.fonts.ready;
    const line = firstChar(range);
    if (!line) return;
    const block = line.startContainer.parentElement;
    const char = line.getClientRects()[0];
    // A paragraph split over two columns has one box per column; take this line's.
    const column =
      [...block.getClientRects()].find((r) => char.left >= r.left - 1 && char.left <= r.right && char.top >= r.top - 1 && char.top <= r.bottom) ||
      char;
    const frame = doc.defaultView.frameElement.getBoundingClientRect();
    const view = viewer.getBoundingClientRect();
    const x = frame.left + char.left;
    if (x < view.left - 1 || x >= view.right) return;

    const area = viewer.parentElement.getBoundingClientRect();
    const left = frame.left - area.left + column.left;
    const top = frame.top - area.top + char.top;
    Object.assign(band.style, { left: left - 8 + "px", top: top - 2 + "px", width: column.width + 16 + "px", height: char.height + 4 + "px" });
    // Above the line when there is room, otherwise just under it.
    const above = top > 34;
    Object.assign(label.style, { left: left - 8 + "px", top: (above ? top - 30 : top + char.height + 6) + "px" });
    if (!label.textContent) label.textContent = viewer.dataset.resumeLabel;
    band.hidden = label.hidden = false;
  };
  if (saved) {
    viewer.parentElement.append(band, label);
    const replace = () => requestAnimationFrame(() => placeResume().catch(() => {}));
    rendition.on("relocated", replace);
    rendition.on("resized", replace);
    rendition.on("rendered", replace);
    // Web fonts arriving after the first measure move the text under the mark.
    rendition.hooks.content.register((contents) => contents.document.fonts.addEventListener("loadingdone", replace));
    addEventListener("resize", () => { band.hidden = label.hidden = true; });
  }

  rendition.display(saved || undefined).then(
    () => {
      placeResume().catch(() => {});
      // Once more when the page has settled, in case the first try came too early.
      setTimeout(() => placeResume().catch(() => {}), 800);
    },
    // A place the book no longer has, after an edit: start from the beginning.
    () => rendition.display(),
  );

  // Percentages and page numbers need the whole book measured, which takes a while
  // for big books, so the measure is kept in this browser as a cache. The place
  // itself is on the account.
  const pagesKey = "reader-pages:" + viewer.dataset.src;
  book.ready
    .then(async () => {
      let cached = null;
      try { cached = localStorage.getItem(locationsKey); } catch {}
      if (cached) {
        book.locations.load(cached);
      } else {
        await book.locations.generate(1600);
        try { localStorage.setItem(locationsKey, book.locations.save()); } catch {}
      }
      try { printed = JSON.parse(localStorage.getItem(pagesKey)); } catch {}
      if (printed === null) {
        printed = await printedPages().catch(() => null);
        // `false` remembers that the book has no printed pages, so it is not searched again.
        try { localStorage.setItem(pagesKey, JSON.stringify(printed || false)); } catch {}
      }
      printed = printed || null;
    })
    .then(() => {
      measured = true;
      progress.disabled = false;
      const where = here();
      if (where) {
        pending = { format: "EPUB", position: where.start.cfi, percent: showPlace(where) };
        save(false);
      }
    });
  progress.addEventListener("input", () => { percentLabel.textContent = progress.value + "%"; });
  progress.addEventListener("change", () => {
    rendition.display(book.locations.cfiFromPercentage(Number(progress.value) / 100));
  });

  // The table of contents, and the current chapter's name in the header.
  const tocList = document.getElementById("toc-list");
  const chapterLabel = document.getElementById("chapter");
  const entries = [];
  const sectionOf = (href) => (href || "").split("#")[0];
  function markChapter(href) {
    const section = sectionOf(href);
    let current = null;
    entries.forEach((entry) => {
      const here = section && (sectionOf(entry.href).endsWith(section) || section.endsWith(sectionOf(entry.href)));
      if (here && !current) current = entry;
      entry.link.removeAttribute("aria-current");
    });
    if (current) {
      current.link.setAttribute("aria-current", "true");
      chapterLabel.textContent = current.label;
    }
  }
  book.loaded.navigation.then((navigation) => {
    const add = (items, depth) =>
      items.forEach((item) => {
        const li = document.createElement("li");
        const link = document.createElement("a");
        link.href = "#";
        link.textContent = item.label.trim();
        link.className = "reader-toc-link";
        link.style.paddingInlineStart = 0.5 + depth * 0.9 + "rem";
        link.addEventListener("click", (event) => {
          event.preventDefault();
          rendition.display(item.href);
          close(panel("toc"), true);
        });
        li.append(link);
        tocList.append(li);
        entries.push({ href: item.href, label: item.label.trim(), link });
        if (item.subitems && item.subitems.length) add(item.subitems, depth + 1);
      });
    add(navigation.toc, 0);
    const where = here();
    if (where) markChapter(where.start.href);
  });
})();
