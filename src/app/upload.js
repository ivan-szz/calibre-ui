// The upload form: lists the chosen files, lights the drop zone while files are
// dragged over it, and shows that the upload is running. The form works without it.
(() => {
  const form = document.querySelector("[data-upload]");
  if (!form) return;
  const input = form.querySelector("input[type=file]");
  const zone = form.querySelector("[data-zone]");
  const list = form.querySelector("[data-files]");
  const submit = form.querySelector("button[type=submit]");
  const label = submit.querySelector("[data-label]");
  const idle = label.textContent;

  const size = (bytes) =>
    bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;

  const render = () => {
    const files = [...input.files];
    list.replaceChildren(
      ...files.map((file) => {
        const row = document.createElement("li");
        row.className = "upload-file";
        const name = document.createElement("span");
        name.textContent = file.name;
        const bytes = document.createElement("span");
        bytes.textContent = size(file.size);
        row.append(name, bytes);
        return row;
      }),
    );
    list.hidden = files.length === 0;
    submit.disabled = files.length === 0;
    label.textContent = idle;
  };

  input.addEventListener("change", render);
  for (const type of ["dragenter", "dragover"]) zone.addEventListener(type, () => (zone.dataset.over = ""));
  for (const type of ["dragleave", "drop"]) zone.addEventListener(type, () => delete zone.dataset.over);
  form.addEventListener("submit", () => {
    // Disabled after this tick, so the browser still sends the form.
    setTimeout(() => (submit.disabled = true));
    label.textContent = submit.dataset.sending;
  });
  // Coming back with the browser's Back button restores the page as it was left.
  addEventListener("pageshow", render);
  render();
})();
