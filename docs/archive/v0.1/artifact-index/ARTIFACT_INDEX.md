# Main Platform Architecture Draft Handoff Bundle v0.1

> **Purpose:** Curated main-platform artifacts for architecture draft generation.  
> **Date:** 2026-06-07  
> **Naming:** OPEN. Use the neutral working label **AI Engineering Control Plane** until naming is reopened.  
> **Important split:** Full Project Brain-specific docs are **not** included here. This bundle includes only `05_PROJECT_BRAIN_INTERFACE/project_brain_platform_interface_notes_v0_1.md`, which is the minimal integration artifact needed for the main platform architecture.

## Recommended read order

1. `01_PRD/main_platform_prd_v0_1.md`
2. `02_PRODUCT_MODEL/platform_product_canon_v0_1.md`
3. `02_PRODUCT_MODEL/shared_object_model_v0_1.md`
4. `03_PLATFORM_MECHANICS/action_gateway_spec_v0_1.md`
5. `03_PLATFORM_MECHANICS/event_model_audit_trail_spec_v0_1.md`
6. `03_PLATFORM_MECHANICS/desktop_first_runtime_addendum_v0_1.md`
7. `04_WORKFLOW_PACKS/workflow_packs_spec_v0_1.md`
8. `04_WORKFLOW_PACKS/cc_crew_workflow_pack_integration_v0_1.md`
9. `05_PROJECT_BRAIN_INTERFACE/project_brain_platform_interface_notes_v0_1.md`
10. `06_UX_DESIGN/platform_ux_information_architecture_spec_v0_1.md`
11. `06_UX_DESIGN/ui_component_inventory_v0_1.md`

## Architecture draft focus areas

- Desktop app shell/runtime split.
- Local runner and process supervision.
- Terminal PTY architecture.
- Claude Code and Codex session adapters.
- Execution Profiles and account/runtime context handling.
- Worktree/git safety model.
- Event store and projections.
- Action Gateway execution pipeline.
- Workflow Pack runtime and personalization.
- Project Brain integration boundary.
- GitHub/Linear integration layer.
- Credential and policy handling.
- iOS companion stretch: observability first, control only through Action Gateway.

## Notes

- The platform is desktop-first, not web-first.
- Duplicate scratch PRD aliases are omitted. The canonical platform PRD is `01_PRD/main_platform_prd_v0_1.md`.
- Project Brain docs are in the separate Project Brain bundle.
