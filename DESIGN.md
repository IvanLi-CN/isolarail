# Design

## Identity

ISO USB Hub documentation uses a restrained hardware-lab identity: pure white reading surfaces, near-black ink, saturated magenta as a sparing brand anchor, and green/cyan technical accents for electrical state and signal language.

## Color

Use OKLCH custom properties in the site stylesheet.

- `--iso-bg`: `oklch(1 0 0)`
- `--iso-surface`: `oklch(0.972 0.006 340)`
- `--iso-ink`: `oklch(0.18 0.018 255)`
- `--iso-muted`: `oklch(0.43 0.026 255)`
- `--iso-primary`: `oklch(0.54 0.19 340)`
- `--iso-primary-strong`: `oklch(0.45 0.18 340)`
- `--iso-accent`: `oklch(0.62 0.15 175)`
- `--iso-caution`: `oklch(0.70 0.14 78)`
- `--iso-line`: `oklch(0.88 0.014 255)`
- `--iso-dark`: `oklch(0.14 0.018 255)`

Primary color is used for identity and the main call to action only. Accent colors mark signal, power, and verification concepts. Avoid cream, beige, navy/orange, and purple-gradient defaults.

## Typography

Use the system sans stack for reliability across Chinese and English:

```css
font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
```

Use a system mono stack only for commands, pin names, addresses, and short technical labels. Do not use monospace as the dominant brand voice.

Display headings use tight but safe letter spacing, never below `-0.03em`. Body copy should remain at 65-75 characters per line on content pages.

## Layout

The home page opens with an asymmetric hero: copy and action links on one side, a real-photo placeholder plus engineering evidence on the other. Interior documentation pages keep Rspress defaults but add clearer hierarchy for callouts, source links, and source-of-truth notes.

Cards are allowed only for distinct navigation choices or repeated document groups. Do not nest cards.

## Components

- Photo placeholder: clearly labeled as awaiting real hardware photography; never styled as a fake product render.
- Evidence strip: compact links to dashboard, hardware overview, software design, and specs.
- Doc group card: title, short audience cue, and one link group.
- Source note: neutral bordered note that names the canonical repository document.
- Warning/caution: full border and background tint, no side stripe.

## Motion

Use subtle page-load emphasis for the home hero and navigation groups. Content must be visible before animation runs. Respect `prefers-reduced-motion: reduce` by disabling transitions.

## Responsive Behavior

The home hero collapses to a single column on narrow viewports. Navigation groups use responsive wrapping rather than fixed breakpoints where possible. Buttons and link clusters must not overflow on mobile.
