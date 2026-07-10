# Product

## Register

product

## Readers

IsolaRail Control serves operators who are already inside a task:

- Bench operators selecting a device and confirming its active transport path.
- Developers and maintainers monitoring live telemetry, port state, and runtime metadata.
- Support and diagnostics users reviewing hardware debug information, device identity, and control-plane status.

Readers stay in this interface for sustained sessions. They need stable control vocabulary, clear state transitions, and high-density information that remains trustworthy under repeated use.

## Product Purpose

The web app is IsolaRail's task surface for device discovery, control, and diagnostics. It should let a user move from "which device am I connected to?" to "what is each rail doing right now?" without context switching into raw logs unless they intentionally choose to.

Success means a reader can answer these questions inside the UI:

- Which device am I looking at and over which transport?
- What is the current connection, health, and power state of each port?
- Can I safely toggle power, replug, or inspect debug information from here?
- Which actions require confirmation, and which states represent warning or fault?

## Brand Personality

Precise, durable, and operator-focused. The app should feel like a calm bench console: familiar enough to trust immediately, disciplined enough to use for hours, and explicit enough that state changes never feel ambiguous.

## Anti-references

- Marketing-style dashboards that prioritize decorative color and hero surfaces over task clarity.
- Terminal cosplay used as a substitute for actual information design.
- Novel control affordances that make standard tasks harder to learn.
- Over-animated panels, transitions, or status indicators that slow down routine operation.

## Design Principles

- Optimize for session length: users should be able to stay in the app for long stretches without visual fatigue.
- Give semantic state first-class treatment: selection, success, warning, error, disabled, and busy states should be obvious.
- Keep control vocabulary consistent: the same kinds of actions should look and behave like siblings everywhere.
- Prefer anchored or inline confirmation before escalating to modal workflows.
- Let dense information stay dense, but never let density become ambiguity.

## Accessibility & Inclusion

The app targets WCAG AA contrast for text and controls in both light and dark themes. Motion must remain optional and state must never rely on color alone. Keyboard focus, disabled states, loading states, and confirmation flows should remain understandable without hover-only cues.
