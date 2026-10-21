// Module worker for lab.html; same startLab() the real worker uses.
import { startLab } from "/src/lab/labHost.js";
let h = () => {};
self.onmessage = (e) => {
  if (e.data.type === "init")
    startLab({
      baseUrl: e.data.baseUrl,
      loadText: async (u) => (await fetch(u)).text(),
      loadBytes: async (u) => (await fetch(u)).arrayBuffer(),
      post: (m) => self.postMessage(m),
      onMessage: (cb) => { h = cb; },
    });
  else h(e.data);
};
