// Import before defining any zod schema. Our CSP has no 'unsafe-eval'; without
// jitless, zod probes `new Function("")` while building object schemas, which
// the browser reports as a CSP violation (zod falls back, but it is noise).
import { z } from "zod";

z.config({ jitless: true });
