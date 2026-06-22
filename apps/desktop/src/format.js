export function emptyToNull(value) {
  const text = String(value ?? "").trim();
  return text ? text : null;
}

export function escapeAttr(value) {
  return escapeHtml(value).replaceAll("'", "&#039;");
}

export function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

export function shortPath(path) {
  const text = String(path ?? "");
  if (text.length <= 34) return text;
  const parts = text.split("/");
  return `.../${parts.slice(-3).join("/")}`;
}
