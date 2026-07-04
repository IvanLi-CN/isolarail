# History

## 2026-07-03

- Created the topic spec to migrate firmware flashing from direct `espflash` / `mcu-agentd` usage to a project-local CLI + devd Local USB flow.
- Implemented the v1 source workflow with minimal firmware identity, `isohub` / `isohub-devd`, Justfile commands, and cargo runner integration.
- Retired the old Makefile, direct `espflash flash --monitor`, and `mcu-agentd` flashing entrypoints.
