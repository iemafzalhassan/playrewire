// A tiny hash router: the static build works from any host or file path, and
// in the Tauri webview, with no server rewrites.
export type Route =
  | { name: "home" }
  | { name: "map" }
  | { name: "settings" }
  | { name: "learn"; pack: string; chunk: string }
  | { name: "missing" };

export function parseHash(hash: string): Route {
  const parts = hash.replace(/^#\/?/, "").split("/").filter(Boolean).map(decodeURIComponent);
  if (parts.length === 0) return { name: "home" };
  if (parts.length === 1 && parts[0] === "map") return { name: "map" };
  if (parts.length === 1 && parts[0] === "settings") return { name: "settings" };
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
