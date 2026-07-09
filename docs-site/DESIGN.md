---
name: IsolaRail Docs
description: Public-facing hardware documentation and product entry for IsolaRail.
colors:
  bg: "#ffffff"
  surface: "#f5f8fb"
  surfaceStrong: "#eef3f8"
  ink: "#11151e"
  muted: "#626d7f"
  line: "#dbe3ed"
  rail: "#090d14"
  primary: "#ff0050"
  accent: "#27c0cf"
  caution: "#efc665"
  darkBg: "#0b1018"
  darkSurface: "#101722"
  darkLine: "#273346"
typography:
  hero:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "clamp(2.8rem, 3.2vw + 1rem, 4.7rem)"
    fontWeight: 900
    lineHeight: 0.9
    letterSpacing: "-0.05em"
  body:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.68
  label:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "0.82rem"
    fontWeight: 800
    lineHeight: 1.45
  mono:
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace'
    fontSize: "0.92rem"
    fontWeight: 700
    lineHeight: 1.45
rounded:
  sm: "8px"
  md: "10px"
  lg: "14px"
  cut: "6px"
spacing:
  xs: "12px"
  sm: "16px"
  md: "20px"
  lg: "24px"
  xl: "30px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "#ffffff"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0 18px"
    height: "44px"
  button-secondary:
    backgroundColor: "{colors.bg}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0 18px"
    height: "44px"
  evidence-tile:
    backgroundColor: "{colors.bg}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "14px 16px"
  doc-card:
    backgroundColor: "{colors.bg}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "18px"
  hero-diagram:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "18px"
  warning-callout:
    backgroundColor: "#fff4db"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "14px 16px"
---

# Design System: IsolaRail Docs

## Overview

Creative North Star: "The Calibrated Lab Note"

This is the public front door for IsolaRail. It should feel like hardware documentation that has been edited into a product surface, not a marketing site that happens to mention hardware. The reader is often beside a bench, a serial console, and open schematics. The page should orient them fast, then stay out of the way.

This surface shares project DNA with the control app, but it is intentionally lighter, flatter, and more public. The relationship should be visible through rail-like framing, restrained magenta markers, cyan proof traces, and tight corner vocabulary. It should not rely on loud typography to prove family resemblance.

**Logo Translation Rule.** The logo informs structure, contrast, bilateral framing, split-bar markers, and proportion. It does **not** license a stylized display face for the main reading hierarchy. Strong logo-like letterforms are accent material only: lockups, micro marks, or occasional route chips.

**Shape Discipline Rule.** Compact UI pieces may not collapse into soft pills. Buttons, evidence tags, and route chips should stay as clipped or hard-edged modules with restrained corners.

## Colors

The docs palette is bright, bench-clean, and precise.

### Primary

- **Isolation Magenta** (`colors.primary`): identity, primary route, and selected emphasis.

### Secondary

- **Trace Cyan** (`colors.accent`): proof, signal path, and technical verification.

### Tertiary

- **Caution Amber** (`colors.caution`): warning surfaces and risk notes only.

### Neutral

- **Bench White** (`colors.bg`): dominant reading field.
- **Frost Surface** (`colors.surface` and `colors.surfaceStrong`): quiet technical shells and structured sections.
- **Graphite Ink** (`colors.ink`): headings and body copy.
- **Measured Muted** (`colors.muted`): supporting text with real contrast.
- **Trace Line** (`colors.line`): borders, grids, and diagram scaffolding.
- **Rail Black** (`colors.rail`): structural linework and logo-adjacent framing.
- **Night Bench** (`colors.darkBg`, `colors.darkSurface`, `colors.darkLine`): dark theme equivalents for after-hours reading.

**Color Discipline Rule.** Magenta is a marker, not a wash. Cyan is proof, not mood. Amber is warning, not decoration.

## Typography

Primary hierarchy uses a neutral system sans. The typography should feel disciplined and trustworthy before it feels branded.

### Roles

- **Hero** (`typography.hero`): route-defining statements and first-fold headings.
- **Body** (`typography.body`): documentation prose and explanations.
- **Label** (`typography.label`): buttons, chips, small titles, and navigational labels.
- **Mono** (`typography.mono`): commands, identifiers, paths, pin names, and measured values.

### Rules

- Main titles, section headings, and card headings stay in neutral sans.
- Heavy stylistic letterforms derived from the logo may appear only as accent moments, never as the primary reading layer.
- Paragraph measure should stay in the comfortable technical-reading range.
- Dark mode may open spacing slightly, but it must not introduce a different typographic personality.

**No-Showoff Rule.** If the page starts feeling like a type specimen, the docs are doing too much.

## Layout and Elevation

The docs site is flat by default. Separation should come from white space, full borders, grid rhythm, and tone shifts.

- Most tiles and cards sit directly on the page with `1px` borders.
- The hero diagram may earn a slightly more instrument-like presence, but it still belongs to the document, not to a floating dashboard world.
- Notes and warnings are full-surface blocks. Colored side stripes are prohibited.

**Flat-First Rule.** If a normal documentation block needs a shadow to read correctly, the layout is underdesigned.

## Components

### Buttons

- **Primary:** magenta fill, white text, restrained corners.
- **Secondary:** white field, graphite text, border-led hierarchy.
- **Rule:** interaction priority should be obvious without theatrical states.
- **Rule:** no full-pill capsules for CTA or evidence tags; compact controls use clipped 6px geometry.

### Evidence Tiles

- Flat, bordered, concise.
- Used for proof links, subsystem entry points, and canonical routes.
- Copy should stay grounded and factual.

### Documentation Cards

- Used only when the topics are genuinely parallel.
- One short title and one grounded sentence.
- Flat surfaces with tight corner discipline.

### Hero Diagram

- Grid-backed, structured, and explicit about hardware boundaries.
- Rails, traces, and labels should read as system explanation, not decorative abstraction.
- This is the one sanctioned place for slightly more visual instrumentation.

### Notes and Warnings

- Full bordered surfaces with explicit tone.
- Warning uses amber tint across the whole block, not a small accent edge.

## Light and Dark Relationship

Light mode is the default public face. Dark mode is a reading adaptation for lower-light bench use.

- Light mode leads with white field and black rails.
- Dark mode flips to carbon field and white rails.
- The two modes keep the same typography, spacing, and information hierarchy.
- Dark mode must not drift into terminal cosplay or sci-fi styling.

## Do's and Don'ts

### Do

- **Do** let neutral typography carry most of the reading load.
- **Do** use logo cues through framing, markers, and rail logic instead of loud headline styling.
- **Do** keep the first fold focused on the hardware boundary before repository structure.
- **Do** keep cards, notes, and tiles flat, bordered, and bench-readable.
- **Do** preserve a clear difference from the app: more air, less density, more public pacing.

### Don't

- **Don't** use a strong logo-like font for hero titles, section titles, or body hierarchy.
- **Don't** use bubbly pill buttons, rounded badges, or soft capsules for core controls.
- **Don't** turn the site into a generic SaaS landing page with soft feature cards and decorative gradients.
- **Don't** use cyan or amber as ambient color washes.
- **Don't** use shadows to fake hierarchy that should come from layout.
- **Don't** let dark mode behave like a different product.
