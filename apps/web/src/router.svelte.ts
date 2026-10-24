// A tiny hash router: the static build works from any host or file path, and
// in the Tauri webview, with no server rewrites.
export type Route =
  | { name: "home" }
  | { name: "map" }
  | { name: "settings" }
  | { name: "skills" }
  | { name: "warmup" }
  | { name: "practise"; concept: string }
  | { name: "fix"; pack: string; chunk: string }
  | { name: "learn"; pack: string; chunk: string }
  | { name: "missing" };

export function parseHash(hash: string): Route {
  let parts: string[];
  try {
    parts = hash.replace(/^#\/?/, "").split("/").filter(Boolean).map(decodeURIComponent);
  } catch {
    return { name: "missing" }; // malformed %-escape: a bad link, not a crash
  }
  if (parts.length === 0) return { name: "home" };
  if (parts.length === 1 && parts[0] === "map") return { name: "map" };
  if (parts.length === 1 && parts[0] === "settings") return { name: "settings" };
  if (parts.length === 1 && parts[0] === "skills") return { name: "skills" };
  if (parts.length === 1 && parts[0] === "warmup") return { name: "warmup" };
  if (parts.length === 2 && parts[0] === "practise") return { name: "practise", concept: parts[1] };
  if (parts.length === 3 && parts[0] === "fix") return { name: "fix", pack: parts[1], chunk: parts[2] };
  if (parts.length === 3 && parts[0] === "learn") return { name: "learn", pack: parts[1], chunk: parts[2] };
  return { name: "missing" };
}

export function href(route: Route): string {
  switch (route.name) {
    case "home":
      return "#/";
    case "map":
      return "#/map";
    case "settings":
      return "#/settings";
    case "skills":
      return "#/skills";
    case "warmup":
      return "#/warmup";
    case "practise":
      return `#/practise/${encodeURIComponent(route.concept)}`;
    case "fix":
      return `#/fix/${encodeURIComponent(route.pack)}/${encodeURIComponent(route.chunk)}`;
    case "learn":
      return `#/learn/${encodeURIComponent(route.pack)}/${encodeURIComponent(route.chunk)}`;
    default:
      return "#/";
  }
}

export function createRouter() {
  const state = $state({ route: parseHash(location.hash) });
  const onChange = () => (state.route = parseHash(location.hash));
  addEventListener("hashchange", onChange);
  return {
    get route() {
      return state.route;
    },
    go(route: Route) {
      location.hash = href(route);
    },
    destroy() {
      removeEventListener("hashchange", onChange);
    },
  };
}
