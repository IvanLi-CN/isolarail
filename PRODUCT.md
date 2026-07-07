# Product

## Register

brand

## Readers

IsolaRail serves three adjacent reading contexts:

- Bench bring-up: toolchain, firmware build, flashing safety, and first hardware checks.
- Engineering orientation: ESP32-S3 control plane, power sequencing, display behavior, and verification gates.
- Project evaluation: what the device is, what is implemented, and where reliable maintenance truth lives.

Readers usually keep the site beside hardware on a desk, with schematics, serial logs, and firmware commands open. They need confident orientation first, then exact instructions.

## Product Purpose

The documentation site is the public web front door for IsolaRail. It explains the device as a product, gives a practical path from setup to bring-up, and organizes long-lived engineering documents without replacing them as source of truth.

Success means a reader can answer these questions without opening the whole repository:

- What does IsolaRail do?
- What hardware and firmware boundaries are currently real?
- How do I install tools, build firmware, and find the right maintenance document?
- Which deeper specs own implementation truth?

## Brand Personality

Precise, bench-ready, and transparent. The voice should feel like a careful hardware lab note that has been edited into a product page: confident enough for public readers, concrete enough for maintainers, and never glossy in a way that hides engineering risk.

## Anti-references

- Generic SaaS landing pages with abstract gradients, hero metrics, and repeated feature cards.
- Decorative electronics imagery that implies a finished enclosure or board photo when no real photo is available.
- Dense internal wiki pages that bury the product story under implementation history.
- Terminal-themed styling used as a shortcut for "technical" rather than because it helps the reader.

## Design Principles

- Show the device boundary before the repository boundary: the first screen should explain the hardware system, not the file tree.
- Be honest about physical evidence: reserve space for real photos and use engineering diagrams only where they are true.
- Keep every route one decision away from action: topic pages lead to setup, hardware topology, firmware runtime, control-plane interfaces, dashboard behavior, or canonical specs.
- Preserve canonical documentation: the site curates and explains, while `docs/` and `docs/specs/` remain long-lived engineering truth.
- Favor clarity over polish: visual style should make sequencing, ownership, and risk easier to scan.

## Accessibility & Inclusion

The site targets WCAG AA contrast for text and interactive controls. Motion must remain optional and respect `prefers-reduced-motion`. Content should not rely on color alone for status or warnings, and technical terms should be readable in both Chinese and English routes.
