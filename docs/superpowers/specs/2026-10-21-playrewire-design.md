# PlayRewire: Design Document

- **Status:** Draft, awaiting approval
- **Date:** 2026-10-21
- **Authors:** Afzal Hassan (product owner), drafted with Claude
- **Shown to players as:** Rewire

> Labels used in this document:
> - **[Owner]**: the product owner decided this.
> - **[Call]**: the drafter recommended it and the owner hasn't rejected it. Review these.
> - **[Open]**: still needs the owner's answer.

---

## 1. Intent: why this exists

Most people who try to learn programming give up early. Syntax looks like noise, error messages feel like punishment, and DSA feels like a wall reserved for "smart people." Platforms like HackerRank, LeetCode, HackerEarth and Codewars assume you already have the foundations. Exercism, Rustlings and the Go Tour are excellent, but they are text-first and give no reason to come back tomorrow.

**PlayRewire is a free, open-source open-world game. Playing it every day turns a complete beginner into someone who understands how computers, networks, operating systems and distributed systems really work, and who writes production-quality code with real muscle memory.** **[Owner]**

The name describes the promise. It rewires how you think, and the world is literally made of wires, networks and kernels.

**Tagline [Call]:** *Rewire: from zero to engineer, one day at a time.*

### Success looks like
- A person who has never coded writes and runs real Go in their **first minute** and isn't afraid.
- Players come back daily because they **want** to, through streaks, story, squads and visible progress, not out of guilt.
- A player who finishes the path can explain how a packet crosses the internet, how the kernel schedules a goroutine's thread, and how Raft elects a leader, and can design and build a production Go service.
- Each earned certificate is trusted, verifiable, and shareable on LinkedIn and through Credly.

---

## 2. Audience

| Phase | Audience | Notes |
|---|---|---|
| **Launch** | **Absolute beginners who are afraid of code**: never coded, or tried and bounced off syntax or DSA **[Owner]** | Everything in v1 is tuned for this person. |
| Later | Kids **[Owner]** | Needs a block-based or no-typing mode and child-safety compliance (COPPA, GDPR-K). |
| Later | Working engineers **[Owner]** | Can skip ahead via placement challenges and use the advanced districts. |

---

## 3. Goals, non-goals, anti-goals

### Goals
1. Teach **real foundations first, interleaved with coding**: hardware, networking, terminal, git, OS and kernel, eBPF, databases, distributed systems, system design, plus Go, DSA, and advanced Go with DSA. **[Owner]**
2. Build **muscle memory** through short daily practice that adapts to each player's mistakes. **[Owner]**
3. Feel like a **real game**: an open world, missions, and **squad mode with friends**. **[Owner]**
4. Run **every platform**: Android, iOS, macOS, Windows, Linux (Debian, RHEL, Arch and others) and the web, with official packages. **[Owner]**
5. **The lab runs on the player's device, never on our servers.** **[Owner]**
6. **One live session per user**, plus a guardrail that knows whether the player is actually active. **[Owner]**
7. Full **gamification**: XP, scores, streaks, and in-game rewards (skins, cars, badges). **[Owner]**
8. **Certificates** players can add to Credly, LinkedIn and other social profiles. **[Owner]**
9. **Personal analytics** showing what you learned and what broke. **[Owner]**
10. **Notification service.** **[Owner]**
11. Backend as **microservices in the owner's own Kubernetes cluster.** **[Owner]**
12. **Fully free and open source**, a sole-owner open-source project, launched on product-launch sites like Product Hunt. **[Owner]**
13. Start with **Go** and be built to add **any language** later. **[Owner]**
14. A **unique, human-made UI**: minimal text, modern, interactive, using AG-UI for the AI mentor. **[Owner]**

### Non-goals (for now)
- Competing with GTA or BGMI on scale, realism or map size. We aim for their *feel*, not their budget. **[Call]**
- Paid tiers, ads or payments of any kind. **[Owner]**
- Real-world cash prizes. **[Owner]**
- A kids' mode or an experienced-engineer fast track in v1. **[Owner]**

### Anti-goals (it fails if this happens, even if it technically works)
- **Dropping beginners into code they haven't been taught.** The game must never ask for something not yet practiced. **[Owner]**
- **Error screens that feel like punishment.** Red stack traces with no guidance count as a bug.
- **Guilt-based engagement.** No dark patterns, no shaming notifications, no streaks you can buy back.
- **Server-side code execution for learning.** If our cluster ends up running practice, mission or squad code, the architecture has failed. The single exception is the small `exam-checker` that verifies certification exams (§7.6). **[Owner]**
- **Pay-to-win**, or rewards that buy learning shortcuts.
- **Generic, template-looking AI-made UI.** **[Owner]**
- **Realistic gun violence.** Action stays stylized (hacking, chases, heists) so the game stays age-friendly and app-store-friendly. **[Call]**

---

## 4. The learning model

### 4.1 The ladder
1. **Spark.** Fear disappears and you run real code in minute one.
2. **Fluency.** Syntax lives in your fingers.
3. **Problem-solving.** DSA as visual puzzles first, then as code.
4. **Building.** Real small programs: a CLI, an HTTP API, a parser.
5. **Production.** Tests, errors, debugging, code review, concurrency, observability.
6. **Engineer thinking.** Tradeoffs, system design, reading other people's code, deciding *what* to build.

### 4.2 The city map is the curriculum **[Owner content, Call naming]**

The open world is split into districts. Each district is one part of the curriculum, and districts unlock in this order:

| # | District | Covers |
|---|---|---|
| 1 | **Silicon Quarter** | What a computer is: CPU (cores, ALU, registers, cache, clock), GPU architectures and why they're massively parallel, motherboard and buses, RAM vs ROM, storage, I/O devices. You walk *inside* a giant computer. |
| 2 | **The Wire** | How the internet works: cables, wires, Wi-Fi, NICs, switches, routers, IP, subnets, DNS, TCP/UDP, the OSI layers, HTTP/TLS. You *become a packet*. |
| 3 | **Terminal Alley** | Shell, files, paths, processes, pipes, permissions, environment variables. |
| 4 | **Git Archive** | Git as a time machine: commits, branches, merges, rebases, remotes, pull requests. |
| 5 | **The Underground** | OS internals: processes, threads, scheduling, virtual memory, syscalls, filesystems. A kernel deep dive, then **eBPF** as "spy cameras inside the kernel." |
| 6 | **Go Academy** | The Go language, from first program to interfaces, errors, generics and modules. |
| 7 | **DSA Arena** | Complexity, arrays, slices, maps, stacks, queues, linked lists, trees, heaps, graphs, sorting, searching, recursion, DP. |
| 8 | **Data Vaults** | Database types and when to use each (relational, document, key-value, columnar, time-series, graph), indexes, transactions and isolation, replication, sharding, the CAP theorem, and **Raft** consensus (as a squad mission where each friend is a node). |
| 9 | **Architect's Tower** | Monolith vs microservices, caching, queues, load balancing, rate limiting, cloud native, containers, Kubernetes. |
| 10 | **The Summit** | Advanced Go with advanced DSA: goroutines and channels deep dive, the runtime and scheduler, profiling, performance, production readiness. |

### 4.3 Interleaving **[Owner]**
Districts unlock in order, but **coding runs from day one**:
- A short daily Go drill runs alongside whichever district you're in.
- Foundation topics are taught *through* tiny Go programs. In the Silicon Quarter you print a number in binary, in The Wire you send a ping or a DNS query, and in The Underground you read `/proc`.

### 4.4 Every concept follows the same four steps **[Owner principle, Call design]**
Every topic is split into small **chunks**. Each chunk goes through:

1. **See.** A short, wordless (or nearly wordless) animation of the idea. No code.
2. **Type.** Guided typing over faded "ghost" code, like a typing tutor, with one-line tooltips on each part.
3. **Drill.** Fast variations from memory with no help. This is the muscle-memory step, and mistakes are recorded.
4. **Use.** A mission in the open world that puts the concept to work.

### 4.5 The help fades **[Call]**
The same concept comes back with less help each time:

| Encounter | Help |
|---|---|
| 1st mission | Fill one blank |
| 2nd mission | Finish half the function |
| 3rd mission | Blank editor plus a hint button |
| Squad heist | Blank editor, hidden tests, only friends' hints |
| Certification exam | Blank editor, no hints, guarded lab |

### 4.6 The daily loop (about 15–20 minutes) **[Call]**
1. **60-second warm-up** targeting yesterday's weak spots, delivered as an in-world side job.
2. **One new chunk** (See → Type → Drill).
3. **Two or three missions** in the city.
4. One **break it / fix it** task: debug something broken. This is where engineering thinking grows.
5. Streak saved, XP awarded, and tomorrow's warm-up scheduled from today's mistakes.

### 4.7 Practice scheduling
The practice scheduler is a spaced-repetition engine keyed by **concept**, not by exercise. It uses an FSRS-style model **[Call]**. Inputs: compile errors, failed tests, hints used, time to solve, retries. Output: tomorrow's warm-up, side jobs, and how much help the next mission gives.

---

## 5. The game

### 5.1 World **[Owner: open world. Call: setting]**
- A stylized open-world city, roamed in third person.
- Interactive objects (traffic lights, ATMs, cameras, drones, vault doors, power grids) are **terminals** you program with real code, and the world reacts instantly.
- Districts appear on the map as unexplored blank paper and get **inked in** as you master them.

### 5.2 Missions **[Owner]**
- Story missions follow a narrative arc. The working premise **[Call]** is that the player is the city's new engineer, and the city's infrastructure has broken, layer by layer, from silicon up to the cloud.
- Side missions come from the practice scheduler (weak spots) and are optional extra practice.
- DSA becomes gameplay. Police chases are shortest-path on the street graph, loading the getaway van is a knapsack problem, sorting loot is sorting, and the vault lock is a stack.

### 5.3 Squad mode **[Owner]**
- Two to four friends take on one shared heist or operation under a shared timer.
- Each player owns one subsystem (lock, cameras, route, alarm) and solves its coding task.
- **Hint pings**: friends can send hints, so players who just learned something teach it.
- The Raft mission: each player *is* a node. They vote, elect a leader, and survive a "network partition."
- Code runs on each player's own device. See §7.6 for verification.

### 5.4 Rewards **[Owner]**
- XP, levels, scores, **daily streaks**, and streak freezes earned through play (never bought).
- **In-game cosmetics only:** character skins, cars, gadgets, badges, and safehouse decorations. Nothing gives a learning advantage.
- Leaderboards: friends, weekly, and per district.

### 5.5 Certification **[Owner]**
- A certificate for each district and one for each ladder rung.
- Earned by passing a **guarded exam** in the local lab: one session, verified as active, no hints, hidden tests.
- Issued as **Open Badges 3.0** verifiable credentials from our own issuer **[Call]**. These can be imported into **Credly**, added to **LinkedIn** "Licenses & Certifications" with a verify URL, and shared on social media. Self-issuing keeps it free. A direct Credly partnership can come later.

### 5.6 The AI mentor **[Call]**
- An optional companion who watches your code in the lab and gives *Socratic* nudges, never full answers.
- Streams into the HUD over the **AG-UI** protocol, so mentor events (thinking, hint, highlight line, celebrate) are a standard event stream the UI renders. **[Owner: AG-UI]**
- **Bring your own model: local or cloud. [Owner]** The player picks a provider in settings:

  | Kind | Providers |
  |---|---|
  | **Local (free, offline, private)** | Ollama, llama.cpp server, LM Studio, or any local OpenAI-compatible server |
  | **Cloud** | NVIDIA NIM, OpenRouter, OpenAI, Anthropic, Meta (Llama API), DeepSeek, MiniMax |
  | **Anything else** | A generic **OpenAI-compatible endpoint** (base URL + key + model), which covers most remaining providers |
  | **Aggregator** | **OmniRoute** preset: points the mentor at a locally running [OmniRoute](https://github.com/diegosouzapw/OmniRoute) gateway (default `http://localhost:20128`), which pools free tiers and signup credits across many providers |

- **How it's built [Call]:** a `MentorProvider` interface in the Rust core with two adapters, *OpenAI-compatible* (most providers, local servers and OmniRoute) and *Anthropic Messages*. Adding a provider is a small adapter or just a preset entry.
- **Where calls run [Call]:** by default from the **player's device** with the **player's own keys**, so the mentor costs our cluster nothing. API keys live in the OS keychain (Keychain, Credential Manager, Secret Service) and are never synced to our servers. The `mentor` service is optional and only needed if an instance operator wants to offer a hosted model.
- **OmniRoute caveats [Call]:** it's off by default and the player installs it themselves. Settings show a short notice that free-tier terms vary by provider and that OmniRoute sees all prompts. Mentor prompts never include personal data, only the current exercise and code.
- **Graceful when absent:** with no provider set, the mentor is replaced by the built-in, non-AI hint ladder that every lesson ships with.

---

## 6. Visual design and UX

### 6.1 Direction: Paper & Ink **[Owner]**
A hand-drawn sketchbook look: wobbly borders, handwritten labels, offset ink shadows, one warm accent color (burnt orange `#e2572b`) on cream paper (`#f6f0e4`) with near-black ink (`#2a2620`).

- **The whole 3D city is hand-inked** **[Call]**: a real-time ink-outline plus flat-watercolor shader. It stands out from realistic AAA games, runs well on low-end phones and the web, and ties menus, lessons and world into one sketchbook.
- Unlocking a district animates as ink spreading across blank paper.

### 6.2 Principles **[Owner: minimal text, unique, interactive. Call: the rest]**
- **One action per screen.** A beginner always knows what to tap.
- **Motion shows cause and effect.** Code runs and the world reacts. Animation is never decoration alone.
- **Errors are friendly.** Every compiler error is translated into plain language and points at the exact spot, with the raw error one tap away.
- **Design tokens** (color, type, spacing, radius, ink-wobble, motion) live in one source shared by the game HUD, the lesson UI and the web app.
- **Accessible from day one:** colorblind-safe palette, scalable text, full keyboard play, a reduced-motion mode that turns off wobble animations, screen-reader labels on all UI, and a dyslexia-friendly font option.
- Handwritten display font for headings, with a highly legible font for body text and code.

---

## 7. Architecture

### 7.1 The core rule **[Owner]**
**All code execution and rendering happen on the player's device. The cluster only stores progress, coordinates squads, and sends signals.**

### 7.2 Client **[Call, owner requested Rust]**

```
┌──────────────────────── Tauri v2 shell (Rust) ────────────────────────┐
│  Web UI (TypeScript, Paper & Ink)                                     │
│   ├─ lessons, menus, HUD, CodeMirror 6 editor                         │
│   └─ <canvas> ← Bevy (Rust → WASM/WebGPU) renders the hand-inked city │
│                                                                       │
│  Rust core (native)                                                   │
│   ├─ lab-runner     real Go toolchain in an OS sandbox (desktop)      │
│   ├─ guard-agent    heartbeat, activity, session lease, exam lockdown │
│   ├─ store          SQLite: progress, content cache, event outbox     │
│   ├─ sync           online/offline sync, content-pack updates         │
│   └─ telemetry      learning events → outbox                          │
└───────────────────────────────────────────────────────────────────────┘
Web build: the same Web UI + Bevy WASM + a Go interpreter (WASM) + IndexedDB
```

- **Why this shape:** one UI codebase for every platform, a real code editor (CodeMirror 6), and Bevy for the 3D world, all in Rust where performance matters.
- **Risk:** Bevy on WebGPU inside mobile webviews. A **performance spike on a mid-range Android phone and an iPhone is the first engineering task.** The fallback is a native Bevy surface beside the webview on mobile.

### 7.3 The local lab **[Owner: local. Call: mechanism]**

| Platform | Engine | Isolation |
|---|---|---|
| Linux | Real Go toolchain, downloaded on first use and pinned | Namespaces, seccomp and Landlock; no network unless the lesson needs it; CPU, memory and time limits |
| macOS | Real Go toolchain | App Sandbox / `sandbox-exec` profile, resource limits |
| Windows | Real Go toolchain | AppContainer plus Job Objects |
| Web, Android, iOS | Go interpreter compiled to WASM (Yaegi is the main candidate) | WASM sandbox. Suits small programs. Limited goroutine and networking realism. |

- Desktop is the "full workshop" for the Building, Production and Summit stages. Phones and the web cover drills, foundations, DSA and most missions. **[Owner accepted]**
- Lessons declare their requirements (`needs: [toolchain, network, goroutines]`) so the client can say "continue this one on desktop" instead of failing.
- The **language runtime is a plugin interface** (`LanguageRuntime`: compile, run, test, explainError), so new languages plug in without changing the core.

### 7.4 Backend services (Go, Kubernetes) **[Owner: microservices in own cluster. Call: service split, Go]**

The backend is written in Go so the codebase can serve as curriculum material in the Summit district.

| Service | Responsibility | Main storage |
|---|---|---|
| `gateway` | Envoy Gateway: TLS, routing, rate limits, auth checks | n/a |
| `identity` | **Keycloak**, already running in the owner's cluster **[Owner]**. Integrated over OIDC (PKCE for the apps). Wired up **after the MVP**; the MVP runs with local-only profiles. | Keycloak's own DB |
| `session-guard` | One live session per user, heartbeat leases, activity state | Valkey |
| `progress` | Chunks completed, mastery per concept, district unlocks | PostgreSQL |
| `practice-scheduler` | Spaced repetition: which concepts to drill when | PostgreSQL |
| `content` | Versioned lesson packs for any language; signed bundles | Object store + CDN |
| `gamification` | XP, streaks, inventory, rewards, leaderboards | PostgreSQL + Valkey |
| `squad` | Friends, parties, matchmaking, state relay, peer-verification coordination | Valkey |
| `credential` | Open Badges 3.0 issuing, signing, verify endpoint | PostgreSQL |
| `notification` | Push (FCM/APNs), email, in-app; quiet hours and frequency caps | PostgreSQL |
| `analytics` | Learning-event ingestion; per-user and per-lesson insights | ClickHouse |
| `exam-checker` | **The only service that runs player code**, and only for certification exams: re-runs a submitted exam solution against hidden tests in a hardened sandbox **[Owner]** | Ephemeral |
| `mentor` | Optional hosted AI mentor and AG-UI event stream; by default the mentor runs on-device (§5.6) | Stateless |

- **Communication:** gRPC (Connect) between client and services, WebSocket or QUIC for squad realtime, and **NATS JetStream** as the event bus between services.
- **Data:** PostgreSQL via **CloudNativePG**, **Valkey** for leases, leaderboards and squad state, **ClickHouse** for analytics, and S3-compatible object storage plus a CDN for content packs, 3D assets and Go toolchains.
- **Platform:** Argo CD (GitOps), OpenTelemetry, Prometheus + Grafana, Cilium (eBPF networking, also used as a live example in The Underground), cert-manager, External Secrets, Kyverno, and a Helm chart for self-hosting.

### 7.5 Session guard and activity **[Owner: requirement. Call: mechanism]**
- Login gives a **session lease** (stored in Valkey, 30-second TTL). The client renews it with a heartbeat every 10 seconds.
- **One live session per user.** Logging in elsewhere asks "Continue here? Your other device will be signed out" and then revokes the old lease.
- **Activity:** the guard agent reports *aggregate* signals only (input in the last N seconds, window focused, lab running). It never records keystroke content. With no activity for 2 minutes, the game pauses and the streak and exam timers stop.
- **Exam mode:** single fullscreen session, focus loss is logged, paste is blocked, hints and the mentor are disabled, and the lease cannot move devices mid-exam.

### 7.6 Verifying results without running code on servers **[Call]**
- **Solo practice:** trust the client. Cheating only cheats yourself.
- **Squad tasks:** the other squad members' clients re-run the solution against the signed hidden tests. A task needs a majority to agree.
- **Certification exams: a small server-side `exam-checker`. [Owner]** The client sends the signed final submission. `exam-checker` re-runs it once against the hidden tests and tells `credential` whether to issue the badge. Its limits **[Call]**:
  - Runs **only** final exam submissions, never practice, missions or squad code.
  - Each run is in a gVisor (or Kata) sandboxed pod with no network, read-only root, and strict CPU, memory and time limits (for example 1 vCPU, 256 MiB, 10 s).
  - Rate limited per user (one check per exam attempt), queued through NATS, and scaled to zero when idle.
  - Exams are small by design, so the cost stays a tiny fraction of what server-side practice would cost.
- Hidden tests are encrypted per attempt and signed by `content`, so they can't be read in advance.

### 7.7 Offline first
- All progress is written to local SQLite first. Events go into an **outbox** and sync when online.
- Conflicts: progress is merge-friendly (sets of completed chunks, max-wins mastery), streaks are computed server-side from synced timestamps, and the client keeps a local copy for display.

### 7.8 Analytics: "what I learned, what broke" **[Owner]**
- Every learning event is tagged with a concept ID: compile error (with its classified error type), test failure, hint used, retry, time-to-solve, chunk completed.
- **For players:** a hand-drawn *skill map* of strong and weak spots, a "most common mistakes" list, and weekly progress.
- **For maintainers:** which chunks confuse people most, drop-off points, and the hint-to-solve ratio for each lesson. This feeds content fixes.
- **Privacy:** no keystroke logs, no code uploaded unless the player opts in, and data export and deletion on request.

### 7.9 Notifications **[Owner: service. Call: rules]**
- Event-driven, for example "streak at risk tonight," "your squad is online," "new district unlocked," "you were *this close* on loops."
- Frequency caps, quiet hours, a per-channel opt-out, and a no-guilt tone.

---

## 8. Content model: any language

- Lessons are **data, not code**: a lesson pack is a versioned directory of Markdown plus YAML (chunk text, ghost code, drill variants, mission bindings, hidden tests, visual specs). **[Call]**
- Packs are signed and distributed as OCI artifacts or object-store bundles, then cached on the device.
- A **new language** means a new `LanguageRuntime` plugin plus a new set of lesson packs. The game, world and services stay the same.
- Lesson content is licensed **CC BY-SA 4.0**, and contributions come in through pull requests that pass automated checks (tests pass, ghost code compiles).

---

## 9. Platforms and packaging **[Owner]**

| Platform | Packages and channels |
|---|---|
| Linux | `.deb` (Debian/Ubuntu), `.rpm` (RHEL/Fedora), AUR + `pacman`, AppImage, Flatpak (Flathub) |
| Windows | `.msi`, winget |
| macOS | `.dmg` (notarized), Homebrew cask |
| Android | Google Play, F-Droid |
| iOS | App Store |
| Web | `play.playrewire.dev` (PWA) |

Releases are reproducible CI builds with signed artifacts (Sigstore/cosign), an SBOM, and SLSA provenance.

---

## 10. Open source and launch **[Owner: fully open, free. Call: specifics]**

- **License:** Apache-2.0 for code, CC BY-SA 4.0 for lesson content, CC0/CC-BY for art assets.
- **Ownership [Owner]:** an independent, **sole-owner** open-source project, not part of any foundation. The owner has final say on direction and releases, and contributions are welcome under the license.
- **Governance:** `GOVERNANCE.md` (owner-led), `MAINTAINERS.md`, the Contributor Covenant Code of Conduct, a DCO sign-off on commits, `SECURITY.md`, and a public roadmap.
- **Funding:** GitHub Sponsors and Open Collective to cover cluster and CDN costs.
- **Self-hosting:** a Helm chart and docs so schools and companies can run their own instance.
- **Foundation listings:** deferred. Nothing foundation-related goes into the repo for now. **[Owner]**
- **Launch sites:** Product Hunt, Hacker News (Show HN), dev.to, Reddit (r/golang, r/learnprogramming), and the Go community channels.

---

## 11. Phasing **[Call]**

| Phase | Ships |
|---|---|
| **0: Spikes** | Bevy-in-webview performance on mid-range phones; Yaegi-WASM lab; Linux sandbox lab-runner; ink shader prototype |
| **1: First playable (MVP)** | Desktop (Linux first) and web; Silicon Quarter + Go Academy basics; See/Type/Drill/Use; local-only profile and progress; XP and streaks; on-device AI mentor with local and cloud providers |
| **2: Online** | **Keycloak** login (wired up after the MVP), session-guard, sync, progress, practice-scheduler, notifications, personal analytics |
| **3: Squads** | Friends, squad heists, peer verification, leaderboards |
| **4: Certified** | Exams, `exam-checker`, Open Badges issuer, LinkedIn/Credly sharing |
| **5: Everywhere** | Android, iOS, macOS, Windows packages; all ten districts complete |
| **6: Grow** | New languages, kids' mode, engineer fast-track |

---

## 12. Owner decisions (resolved 2026-10-21)

1. **Exam verification:** a small server-side `exam-checker`, used only for certification exams. (§7.6)
2. **Identity provider:** Keycloak, already running in the cluster, wired up after the MVP. (§7.4, §11)
3. **AI mentor:** local models plus NVIDIA NIM, OpenRouter, OpenAI, Anthropic, Meta, DeepSeek, MiniMax, any OpenAI-compatible endpoint, and an OmniRoute preset. (§5.6)
4. **Foundation listings:** deferred; the repo stays an independent, sole-owner project. (§10)

No owner decisions are open right now.

## 13. Left to the builder
- Exact service boundaries inside the table in §7.4 (it's fine to merge services early and split later).
- Choice of ink-shader technique, physics, and character controller.
- Lesson-pack schema details.
- Specific spaced-repetition parameters.
- CI layout and repo structure (a monorepo is recommended).
