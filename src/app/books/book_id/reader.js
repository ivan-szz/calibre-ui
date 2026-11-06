// EPUB reader on epub.js. Remembers the reading position per book in this browser.
(() => {
  const viewer = document.getElementById("viewer");
  if (!viewer || !window.ePub) return;

  const book = ePub(viewer.dataset.src, { openAs: "epub" });
  const rendition = book.renderTo(viewer, { width: "100%", height: "100%", spread: "auto" });
  const key = "reader-position:" + viewer.dataset.src;

  if (matchMedia("(prefers-color-scheme: dark)").matches) {
    rendition.themes.default({ body: { color: "#e6e7eb !important", background: "transparent !important" } });
  }

  let saved;
  try { saved = localStorage.getItem(key) || undefined; } catch {}
  rendition.display(saved);
  rendition.on("relocated", (location) => {
    try { localStorage.setItem(key, location.start.cfi); } catch {}
  });

  document.getElementById("prev").addEventListener("click", () => rendition.prev());
  document.getElementById("next").addEventListener("click", () => rendition.next());
  const onKey = (event) => {
    if (event.key === "ArrowLeft") rendition.prev();
    if (event.key === "ArrowRight") rendition.next();
  };
  document.addEventListener("keydown", onKey);
  rendition.on("keydown", onKey);
})();
