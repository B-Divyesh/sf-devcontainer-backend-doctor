# Visual thesis — “The night crossing”

## Direction and rationale

Devcontainer Backend Doctor uses **cinematic environmental art**: a container ship crossing a dark, fog-bound strait while four harbor lights mark possible runtimes. The world turns abstract compatibility work into a concrete pre-departure inspection. It feels operational and calm, not alarmist: the CLI is the chart table that finds hazards before a team leaves a working harbor.

This is intentionally a single dark treatment. A midnight operations room keeps the hero art, terminal demo, and diagnostic states in one coherent visual world. Surfaces are painted explicitly; the interface never depends on browser theme defaults.

## Palette

| Token | Value | Role |
| --- | --- | --- |
| `night-950` | `#071014` | page background / deep water |
| `night-900` | `#0b171b` | raised surface |
| `night-800` | `#13252a` | rules and terminal chrome |
| `mist-50` | `#f0f4ed` | primary text |
| `mist-300` | `#aab9b4` | supporting text |
| `signal` | `#e8b44f` | primary action / inspection lamp |
| `signal-ink` | `#171205` | text on signal |
| `safe` | `#78c79a` | compatible/pass |
| `warn` | `#f0b85c` | portability risk |
| `danger` | `#ef7b68` | known incompatibility |
| `chart` | `#80c7d2` | links / informational markers |

All text and controls meet WCAG AA against their assigned surfaces. Status always includes a word or symbol, never color alone.

## Typography

- **Headings:** Georgia with the platform serif fallback. Its editorial, chart-room character carries the environmental story without a font download.
- **Interface and body:** ui-monospace / SFMono-Regular / Menlo / Consolas. The product is a CLI, so command syntax and explanatory prose share a precise technical cadence.
- Scale: 14, 16, 20, 28, and clamp(42–72) px. Body stays at 16 px minimum, with 1.55 line height and a 68-character reading measure.

No remote or bundled font files are needed, keeping font transfer at 0 KB.

## Spacing and composition

An 8 px base rhythm with 4 px for optical adjustments. Major sections use 80–128 px vertical air on desktop and 64–80 px on mobile. The opening composition is asymmetrical: copy occupies the lit chart-table foreground while the original seascape supplies depth. At 390 px the scene becomes an atmospheric top panel, runtime chips wrap, and all utility comparisons stack.

## Interaction grammar

- Primary actions glow like a lamp being switched on: a crisp amber surface, 2 px downward press, no bloom.
- Diagnostic rows reveal left-to-right because an inspection proceeds down a manifest.
- Terminal tabs behave like physical labels and expose selected state in text and border.
- Focus is a 3 px mist-and-amber double ring. Every target is at least 44 px high.

## Motion policy

UI transitions last 180–240 ms and affect only opacity and transform. The hero image has one subtle, finite 700 ms reveal; terminal results step in once after a demo selection. Nothing loops. Under `prefers-reduced-motion: reduce`, all transforms and smooth scrolling are removed and state changes are immediate.

## Original asset plan and provenance

- `site/public/harbor-crossing.webp`: generated specifically for this product with the factory image deployment, then resized and converted locally to WebP. Used as meaningful hero imagery with descriptive alt text. No stock assets, logos, or third-party material.
- Prompt (verbatim): “Use case: stylized-concept. Asset type: wide landing page hero for a developer CLI. Primary request: cinematic environmental concept art of a compact cargo vessel paused before crossing a dark foggy strait, seen from a high coastal chart-room viewpoint; four distant harbor beacons represent alternative container runtimes and a warm inspection lamp illuminates a paper navigation chart in the near foreground. Style/medium: restrained painterly cinematic matte painting, realistic atmosphere, subtle film grain, no science-fiction interface. Composition/framing: wide 3:2 landscape, vessel and strait on the right two thirds, quiet negative space and darker values on the left for headline legibility. Lighting/mood: blue-black pre-dawn, low mist, one amber practical light, calm and vigilant rather than ominous. Color palette: deep ink teal, oxidized blue, bone mist, amber signal light. Constraints: no people, no logos, no brand marks, no text, no letters, no UI screens, no watermark; the vessel must read as a shipping container vessel rather than a warship.”
- Generator: `/opt/fleet/lib/gen-image.sh`, factory `factory-image` deployment, 1536×1024, high quality, generated 2026-08-27. License: original commissioned project asset, distributed under the repository MIT license.

