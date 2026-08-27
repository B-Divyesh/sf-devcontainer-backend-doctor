# Devcontainer Backend Doctor

Predict whether a devcontainer or Compose project will survive a Mac container-runtime switch **before** the first launch. The doctor reads project files locally, compares them with a version-aware compatibility rule set, and can optionally probe an installed backend without starting or changing containers.

Built for teams evaluating Docker Desktop, Podman, OrbStack, or Apple Container. No telemetry, daemon mutation, or secret-value output.

## Install

Prebuilt binaries will be attached to releases. From source:

```sh
cargo install --path crates/devcontainer-backend-doctor
```

Requires Rust 1.85 or later. The binary is named `devcontainer-backend-doctor`; `dcdoctor` is a convenient shell alias if you want one.

## Usage

Check the current repository against a backend:

```sh
devcontainer-backend-doctor check . --backend podman
```

Point it at either a project directory or a specific `devcontainer.json` / Compose file:

```sh
devcontainer-backend-doctor check ./service --backend orbstack
devcontainer-backend-doctor check .devcontainer/devcontainer.json --backend apple-container
```

Probe the selected local backend as well as inspecting files (explicit opt-in):

```sh
devcontainer-backend-doctor check . --backend docker-desktop --probe
```

Machine-readable output and strict CI behavior:

```sh
devcontainer-backend-doctor check . --backend podman --json
devcontainer-backend-doctor check . --backend apple-container --fail-on warning
```

Supported backends are `docker-desktop`, `podman`, `orbstack`, and `apple-container`.

### Exit codes

| Code | Meaning |
| ---: | --- |
| 0 | Inspection completed; no finding met the configured failure threshold |
| 1 | A finding met `--fail-on` (`error` by default) |
| 2 | Invalid arguments, unreadable input, or malformed project configuration |
| 3 | Requested backend probe could not run or identify a usable backend |

The JSON schema is versioned with `"schema_version": 1`. Findings expose stable rule IDs, severity, source location, message, evidence, and remediation. Secret values are never included.

## What it checks

- Bind mounts, Docker socket mounts, consistency flags, and non-portable host paths
- Privileged mode and Linux capability additions
- Host networking and `host.docker.internal`
- Explicit `platform` / architecture constraints and host architecture mismatches
- NVIDIA and Compose GPU requests
- Devcontainer and Compose secrets (names and missing-file conditions only)
- Backend availability, version, architecture, and connection health with `--probe`

This is a static portability diagnosis. It never starts containers, edits daemon settings, or claims runtime equivalence where a backend lacks a feature.

## Development

```sh
npm ci
npm test
npm run build
```

`npm test` runs Rust tests plus site tests. `npm run build` compiles a release binary, builds the static site into `dist/site/`, and stages the binary under `dist/bin/`. To work only on the site, use `npm run dev` or `npm run build:site`.

Create a registry-ready Rust package without publishing:

```sh
cargo package --manifest-path crates/devcontainer-backend-doctor/Cargo.toml
```

## Privacy and security

All analysis runs locally. Probes execute only when requested, are read-only (`version` / `info` style commands), and have a short timeout. Output redacts secret-like values and environment assignments. See [the site privacy page](site/privacy.html) for the full plain-language policy.

## Deployment

The factory deploys `dist/site/` to `https://devcontainer-backend-doctor.sociobot.in`. This repository does not manage DNS, billing, or production infrastructure.

## License

MIT. See [LICENSE](LICENSE).
