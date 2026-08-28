# Devcontainer Backend Doctor — v0.1.0 handoff

## Independent verification status — PASS (2026-08-28)

Candidate `f6d149a0e64a956cf8106b70cf38fee88d1f5cc3` was independently tested from a clean detached checkout and against <https://devcontainer-backend-doctor.sociobot.in/>. **PASS — no Critical, High, Medium, or Low defects found.**

Fresh verification passed `npm ci`, `npm test`, `npm run check`, the exact production command `npm run build`, Playwright desktop and 390 px mobile tests, `cargo package`, clean-consumer installation, and the live website's documented `cargo install --git` command. The production HTML and hashed JS are byte-identical to the candidate build; live axe had 0 serious/critical findings, console/page errors were 0, offline service-worker reload passed, Lighthouse scored 100/100/100/100, and first-load transfer was 58 KiB. Full commands, exit-code fixtures, privacy/request/header checks, response caching, bundle sizes, and the only non-blocking Playwright browser-install observation are in `.factory/verification.md`.

## What shipped

- A Rust single-binary CLI with a small public library API and `clap` command surface.
- JSONC `devcontainer.json` discovery, linked Compose discovery, YAML parsing, source lines, stable rule IDs, human reports, and schema-versioned JSON reports.
- Backend-specific diagnosis for Docker Desktop, Podman, OrbStack, and Apple Container across mounts/socket use, privileged mode/capabilities, networking, architecture, GPU, and secrets.
- Explicit `--probe` behavior using only short, read-only status/version commands. Probe errors return exit code 3; static checks never invoke a backend. Docker host-networking rules incorporate the probed version threshold.
- CI controls through `--json` and `--fail-on info|warning|error`, with documented exit codes.
- A responsive static docs site with an original cinematic hero, keyboard-operable recorded diagnosis, install/reference content, offline service worker, `/privacy/`, and `/terms/`.
- A product-specific design record in `.factory/design.md`, README, MIT license, changelog, caching/security headers, tests, and package metadata.

## Run and verify

From a clean clone with Rust 1.85+, Node 20+, and npm:

```sh
npm ci
npm test
npm run build
```

The exact production build command is `npm run build`. It writes the deployable static site to `dist/site/` (with `index.html` at that root) and the host release binary to `dist/bin/devcontainer-backend-doctor`.

Additional checks run for this handoff:

```sh
npm run check
npx playwright install chromium
npm run test:browser
cargo package --manifest-path crates/devcontainer-backend-doctor/Cargo.toml --allow-dirty
```

Results on 2026-08-27:

- Rust: 6 unit tests + 2 CLI integration tests passed; clippy passed with warnings denied; rustfmt clean.
- Site: 3 source-contract tests passed; 4 Playwright tests passed across desktop Chromium and a 390×844 viewport.
- Accessibility: 0 serious/critical axe findings; keyboard tab/arrow interaction, semantic landmarks, one h1, alt text, legal routes, and console errors were checked in-browser.
- Production Lighthouse mobile: Performance **100**, Accessibility **100**, Best Practices **100**, SEO **100**; LCP **1.4 s**, TBT **0 ms**, CLS **0**, Speed Index **1.0 s**, total transfer **58 KiB**.
- Production assets: initial JS 5.02 KB (2.03 KB gzip), CSS 12.68 KB (3.55 KB gzip), hero WebP 49.2 KB; no fonts or third-party runtime requests.
- Rust package: `cargo package` produced and verified a 17.1 KiB compressed crate. It was not published.

## Asset provenance

`site/public/harbor-crossing.webp` was generated with `/opt/fleet/lib/gen-image.sh` using the factory `factory-image` deployment, then resized to 1440×960, stripped, and encoded as WebP at 49.2 KB. The exact prompt, rationale, palette, and license are recorded in `.factory/design.md`. The unoptimized generation source is excluded from version control.

## Known gaps and next steps

- The product is ready for pilot use, but the brief’s “90% of runtime-switch failures” target needs real pilot incident data; no honest measured recall is available yet. Add anonymized rule fixtures only with explicit user consent.
- Apple Container and editor integration are evolving. The v0.1 rule set intentionally treats its current lack of stable Dev Containers/Compose support as blocking and should be revised when those upstream capabilities ship.
- Live runtime probes could not be exercised against all four macOS backends inside this Linux build worker. Their command selection, timeout/failure path, normalization, opt-in boundary, and version comparison are covered by code review/tests; run the CLI on target Macs before publishing release binaries.
- `dist/bin/` contains the build-worker’s Linux binary. The factory should cross-build/notarize release binaries for macOS arm64 and amd64; registry and release publishing remain factory-owned.
