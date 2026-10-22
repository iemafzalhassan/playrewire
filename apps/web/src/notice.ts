// One-line notices from the platform layer to the shell (e.g. storage loss).
type Listener = (text: string) => void;
const listeners = new Set<Listener>();
const shown = new Set<string>();

/** Show `text` once per window, however many times it is reported. */
export function notifyOnce(text: string) {
  if (shown.has(text)) return;
  shown.add(text);
  for (const l of listeners) l(text);
}

export function onNotice(l: Listener): () => void {
  listeners.add(l);
  for (const t of shown) l(t);
  return () => listeners.delete(l);
}
