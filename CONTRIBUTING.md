# Contributing to PlayRewire

Thanks for helping people learn to build. Every contribution counts, including code, lessons, art, translations, bug reports and design feedback.

## Ways to contribute

| You are… | Start here |
|---|---|
| An engineer | Issues labeled [`good first issue`](https://github.com/iemafzalhassan/playrewire/labels/good%20first%20issue) or [`help wanted`](https://github.com/iemafzalhassan/playrewire/labels/help%20wanted) |
| An educator or writer | Issues labeled [`area/curriculum`](https://github.com/iemafzalhassan/playrewire/labels/area%2Fcurriculum) |
| An artist | Issues labeled [`area/art`](https://github.com/iemafzalhassan/playrewire/labels/area%2Fart) |
| A translator | Issues labeled [`area/i18n`](https://github.com/iemafzalhassan/playrewire/labels/area%2Fi18n) |
| A player | Open a bug report or start a discussion |

## Before you start

1. **Read the design doc:** [docs/superpowers/specs/2026-10-21-playrewire-design.md](docs/superpowers/specs/2026-10-21-playrewire-design.md).
2. **For anything bigger than a small fix, open an issue first** so we can agree on the approach before you invest time.
3. Be kind. This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Developer Certificate of Origin (DCO)

Every commit must be signed off to certify that you wrote it or have the right to submit it under the project's license ([developercertificate.org](https://developercertificate.org/)):

```bash
git commit -s -m "feat(lab): add Landlock policy for Linux runner"
```

This adds a `Signed-off-by: Your Name <you@example.com>` line. Pull requests with unsigned commits can't be merged.

## Pull request workflow

1. Fork the repo and create a branch: `git checkout -b feat/short-description`.
2. Make focused changes. One logical change per pull request.
3. Use [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
4. Add or update tests and docs for what you changed.
5. Open the pull request using the template and link the issue (`Closes #123`).
6. A maintainer reviews it. Expect discussion, since it's how we keep quality high.

## Lesson content

Lessons are **data, not code**. Each chunk follows **See → Type → Drill → Use**:

- **See:** a short visual explanation with almost no text.
- **Type:** ghost code the learner types over, with one-line tooltips.
- **Drill:** at least 5 variations to solve from memory.
- **Use:** a mission binding with hidden tests.

Content guidelines:

- Write for someone who has **never coded and is a little scared**. Use plain words and short sentences.
- One idea per chunk.
- Every ghost-code and drill solution must compile and pass its tests, and CI checks this.
- Lesson content is licensed **CC BY-SA 4.0**.

## Reporting bugs and security issues

- **Bugs:** use the bug report template.
- **Security vulnerabilities:** **don't** open a public issue. Follow [SECURITY.md](SECURITY.md).
