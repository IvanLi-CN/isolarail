---
name: IsolaRail Control
description: Task-focused control surface for device discovery, telemetry, and per-port power operations.
colors:
  bg: "#e8edf4"
  panel: "#ffffff"
  panel2: "#f4f7fa"
  panel3: "#dfe7ef"
  border: "#cad4e0"
  text: "#131925"
  muted: "#5f6a7c"
  primary: "#ff0050"
  primaryText: "#ffffff"
  trace: "#4fc3d0"
  success: "#18a46c"
  warning: "#c57a1b"
  error: "#d45555"
  darkBg: "#091018"
  darkPanel: "#101722"
  darkBorder: "#243143"
typography:
  headline:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "1rem"
    fontWeight: 800
    lineHeight: 1.3
  body:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "0.95rem"
    fontWeight: 400
    lineHeight: 1.55
  label:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "0.76rem"
    fontWeight: 800
    lineHeight: 1.4
  mono:
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace'
    fontSize: "0.95rem"
    fontWeight: 700
    lineHeight: 1.35
rounded:
  sm: "8px"
  md: "10px"
  lg: "12px"
  xl: "14px"
  cut: "6px"
spacing:
  xs: "12px"
  sm: "16px"
  md: "20px"
  lg: "24px"
  xl: "28px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primaryText}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "40px"
  button-outline:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0 14px"
    height: "40px"
  device-card:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
    padding: "14px"
  port-card:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
    padding: "14px"
  status-chip:
    backgroundColor: "{colors.panel2}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
  dialog-panel:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    rounded: "{rounded.xl}"
    padding: "18px"
---

# Design System: IsolaRail Control

## Overview

Creative North Star: "The Relay Console"

This is the operator surface for IsolaRail. It is not the public front door. It is where someone selects a device, verifies a route, reads telemetry, and performs power-related actions repeatedly without losing trust in the interface.

It must visibly belong to the same project as the docs site, but it should express that relationship through structural cues rather than showy typography. Shared DNA comes from rails, marker logic, tight corners, restrained magenta intent, cyan proof traces, and a disciplined neutral sans foundation.

**Logo Translation Rule.** The logo influences route framing, paired bars, bilateral composition, and accent markers. It does **not** justify using a strong display-style font for major headings, card titles, or core UI labels. Distinctive logo-like letterforms are accent-only material.

**Shape Discipline Rule.** Controls, tabs, and state chips must read as engineered modules, not soft bubbles. Use clipped or hard-edged compact geometry instead of full-pill capsules.

## Colors

The app palette is denser and more operational than the docs surface.

### Primary

- **Intent Magenta** (`colors.primary`): primary action, selected route, and explicit operator intent.

### Secondary

- **Trace Cyan** (`colors.trace`): measurement, proof, and verified system path.
- **State Green** (`colors.success`): healthy or active runtime condition.
- **Alert Amber** (`colors.warning`): queued, cautionary, or non-ideal state.
- **Fault Red** (`colors.error`): error and destructive state only.

### Neutral

- **Shell Mist** (`colors.bg`): overall light console field.
- **Panel White** (`colors.panel`): main module surface.
- **Panel Frost** (`colors.panel2`) and **Panel Shell** (`colors.panel3`): secondary layers and navigation framing.
- **Trace Border** (`colors.border`): module edges and internal dividers.
- **Graphite Text** (`colors.text`): primary UI copy.
- **Slate Muted** (`colors.muted`): secondary labels and support lines.
- **Night Shell** (`colors.darkBg`, `colors.darkPanel`, `colors.darkBorder`): dark-mode operator equivalents.

**Accent Budget Rule.** Magenta is for intent and active isolation state. It is not a general chrome color.

**Semantic Separation Rule.** Green, amber, and red may never be repurposed as branding.

## Typography

The app uses a neutral system sans for all core hierarchy. This surface should feel stable and task-ready, not typographically expressive.

### Roles

- **Headline** (`typography.headline`): page titles, module titles, and device labels.
- **Body** (`typography.body`): explanations, secondary metadata, and system notes.
- **Label** (`typography.label`): buttons, tabs, chips, and compact control labels.
- **Mono** (`typography.mono`): telemetry, route identifiers, state strings, and measured values.

### Rules

- Major headings and card titles stay in neutral sans.
- Any logo-like cut or high-character letterform is accent-only and cannot carry the main information layer.
- Mono is important on this surface because trust often comes from values and identifiers.
- Product UI stays on fixed rem sizing and compact scale. No display drama.

**No-Specimen Rule.** If the operator notices the typography before the state model, the UI is overstyled.

## Shape and Density

The app is tighter and denser than the docs site.

- Corners stay in the `8px` to `14px` range.
- Modules are compact and rectangular rather than pillowy.
- Layout should support more information per viewport than the docs surface, but never collapse into ambiguity.
- Sidebars, route strips, and telemetry cells are allowed to be dense as long as labels, values, and actions remain obvious.

**Machine-Flat Rule.** The app should feel engineered, not cushioned.

## Elevation

This surface can use a limited depth model, but only in service of legibility.

- Panels may lift slightly from the shell.
- Popovers and dialogs can step above the base layers.
- Depth supports task clarity; it should never become soft dashboard theater.

## Components

### Buttons

- **Primary:** magenta fill, white text, explicit intent.
- **Outline:** neutral panel with border-led hierarchy.
- **Rule:** primary and neutral actions should remain clearly separable even in dense modules.

### Device Cards

- Compact metadata, route chips, and action entry points.
- Tight corners and strong internal borders.
- Selected state may use magenta emphasis, but the card should not flood with brand color.

### Port Cards

- Dense telemetry cells with label-value clarity.
- Mono values are expected.
- Avoid ornamental gauges when a compact numeric readout is clearer.

### Status Chips

- Small, legible, and semantically explicit.
- Intent chips, trace chips, and state chips must not collapse into one visual role.
- Shape stays hard-edged or clipped; no round tag-cloud treatment.

### Dialogs and Confirmation

- Prefer inline or anchored confirmation before modal escalation.
- If a dialog is necessary, it should stay sharp and operational rather than theatrical.

## Light and Dark Relationship

Light and dark are the same console under different ambient conditions.

- Light mode uses mist shell and dark rails.
- Dark mode uses carbon shell and white rails.
- Typography, density, and action model stay consistent.
- Dark mode must not drift into sci-fi or terminal cosplay.

## Relationship to Docs

The docs site and app are clearly siblings, but they do not behave the same way.

- **Docs:** flatter, airier, more public, slower pacing.
- **Web:** denser, more modular, more action-oriented.
- Shared cues should come from rails, markers, spacing discipline, and accent logic.
- Shared cues should **not** come from reusing a loud stylized font in both places.

## Do's and Don'ts

### Do

- **Do** keep the core hierarchy neutral and practical.
- **Do** reserve magenta for operator intent and active route emphasis.
- **Do** make telemetry and state scannable before making the UI expressive.
- **Do** keep corners, spacing, and module rhythm tighter than the docs surface.
- **Do** let the family resemblance come from structure and accent logic.

### Don't

- **Don't** use a strong logo-like font for page titles, module titles, or core labels.
- **Don't** use pill chips, round badges, or soft console controls.
- **Don't** reintroduce generic SaaS indigo-dashboard styling.
- **Don't** turn dark mode into a fantasy control room.
- **Don't** let semantic states borrow brand color jobs.
- **Don't** use soft oversized cards or decorative widgetry where a tighter module would read better.
