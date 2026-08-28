# Independent verification — PASS

- **Verified:** 2026-08-28
- **Candidate commit:** `f6d149a0e64a956cf8106b70cf38fee88d1f5cc3`
- **Live URL:** <https://devcontainer-backend-doctor.sociobot.in/>
- **Method:** fresh detached worktree at the candidate commit; no product files changed during verification.

## Verdict

**PASS.** The CLI fulfills the brief's smallest useful job: it statically inspects Dev Container and Compose input locally, reports backend-specific incompatibilities across the required categories, keeps secret values out of output, and invokes a backend only with explicit `--probe`. The live site is the tested candidate build; the purported deployment-only failure is not reproducible.

## Clean-checkout quality gates

Environment: Node `v22.23.2`, npm `10.9.8`, Rust `1.98.0` (candidate requires Rust 1.85+).

| Command | Result |
| --- | --- |
| `npm ci` | Passed; 21 packages audited, 0 vulnerabilities. |
| `npm test` | Passed: 6 Rust unit, 2 Rust CLI integration, and 3 site source-contract tests. |
| `npm run check` | Passed: `cargo fmt --check`, Clippy with `-D warnings`, then the full `npm test` suite. No separate JS type/lint script is declared. |
| `npm run build` | Passed. Produced `dist/site/` and `dist/bin/devcontainer-backend-doctor` (1,900,848 bytes, Linux build-worker binary). |
| `npm run test:browser` | Passed: 4 Playwright tests over desktop Chromium and 390×844 mobile. |
| `cargo package --manifest-path crates/devcontainer-backend-doctor/Cargo.toml --allow-dirty` | Passed and verified: 17.5 KiB compressed crate. |

`npm ci` resolves Playwright 1.62.1, while the supplied preinstalled Chromium revision targeted a different Playwright version. After the documented `npx playwright install chromium` prerequisite, the browser suite passed. This is a non-blocking test-environment reproducibility observation, not a product defect.

## CLI end-to-end evidence

I exercised the release binary and a clean consumer installation using purpose-made local fixtures.

- Clean Compose input with each normal backend path: JSON report, no findings, exit `0`.
- JSONC `devcontainer.json` linked to Compose containing Docker socket and absolute mounts, mount consistency, privileged mode, capabilities, host networking/gateway, arm64 platform pin, GPU, declared secret, and missing secret file: Apple Container report had the expected blocking findings and exit `1` (17 errors, 6 warnings). The deliberate fixture secret value was absent from the full JSON output.
- `--fail-on info` correctly returned exit `1`; malformed YAML and a nonexistent path returned JSON `invalid_project` and exit `2`; an unknown backend was rejected with exit `2` and actionable help.
- Opt-in-only `--probe` against an unavailable Podman executable returned JSON `probe_failed` and exit `3`; unprobed inspections did not require or contact a backend.
- The public help text states usage, examples, all supported backends, `--json`, and exit codes.
- `cargo install --path ... --root <empty-consumer>` installed the binary, which successfully inspected a clean consumer fixture. The exact website command, `cargo install --git https://github.com/B-Divyesh/sf-devcontainer-backend-doctor`, also resolved and installed commit `f6d149a0` and its binary successfully exercised the public CLI.

## Live deployment, privacy, and web QA

The live `/` HTML and `assets/home-DAzVFJsa.js` SHA-256 hashes are byte-for-byte identical to the fresh `dist/site` build. The deployed CSS, hero WebP, privacy page, terms page, and service worker also match their built artifacts. `staticwebapp.config.json` is deployment configuration and is correctly consumed rather than publicly served.

- Desktop and 390 px mobile: no horizontal overflow (`scrollWidth == clientWidth == 390`); the recorded diagnosis tabs work with keyboard Arrow keys; the first Tab reaches the skip link; focused tabs show a 3 px high-contrast outline and 6 px ring; targets sampled at 52 px high.
- Accessibility: live axe scan found **0 serious/critical** findings on both desktop and mobile. The site has language, title, one H1, main landmark, image alt text, legal routes, and reduced-motion behavior (`transitionDuration` effectively zero under `prefers-reduced-motion: reduce`). No console or page errors occurred.
- PWA: the production service worker activated and controlled the page after reload; with the browser offline, a subsequent reload displayed the cached home page successfully. Its cache versioning removes old caches during activation.
- Privacy/outbound requests: browser traffic stayed entirely on `https://devcontainer-backend-doctor.sociobot.in`; there are no third-party fonts, scripts, analytics, advertising, or cookies. The privacy policy accurately states this local-first behavior.
- Response policies: HTTPS index returned HSTS, CSP (`default-src 'self'`), `Permissions-Policy`, strict-origin referrer policy, and `X-Content-Type-Options: nosniff`. Hashed JS/CSS used `public, max-age=31536000, immutable`; HTML, legal routes, and service worker used short revalidation; hero WebP used one-week caching.
- Bundle budgets: JS 5,018 B (2,046 B gzip), CSS 12,677 B (3,566 B gzip), no downloaded fonts, hero 49,200 B; all within the stated static-product budgets.
- Live Lighthouse (mobile configuration): Performance **100**, Accessibility **100**, Best Practices **100**, SEO **100**; FCP 1.2 s, LCP 1.2 s, TBT 80 ms, CLS 0, Speed Index 1.2 s, transfer 58 KiB.

## Defects by severity

| Severity | Defects |
| --- | --- |
| Critical | None |
| High | None |
| Medium | None |
| Low | None |

## Scope limits

The disposable Linux verifier cannot operate Docker Desktop, Podman machine, OrbStack, or Apple Container on macOS. Their live probe commands, opt-in boundary, successful-output normalization, failure path, timeout, and Docker version rule were covered by the executable and automated tests; target-Mac runtime probe validation remains a sensible pre-release field check, not a blocker for this static-analysis CLI.
