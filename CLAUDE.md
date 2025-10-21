<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

# Claude Playbook

## Mission Brief

- Repository: `tevm/compiler` – Rust + N-API bridge exposing Foundry's compiler to JS/Bun/WASI. Core exports: `Compiler`, `Ast`, `Contract`, `CompileOutput`.
- Primary goal: keep the TypeScript-facing API reliable, accurately documented, and paired with real build/test coverage. Most changes require touching Rust + TS glue.

## Operating Procedure

1. **Assess**
   - Run `pnpm nx graph --focus=compiler` when you need project dependencies.
   - Read `libs/compiler/src/**/*.rs` alongside `build/index.d.ts` to understand the surface area impacted by any change.
2. **Modify**
   - Prefer `pnpm nx run compiler:build` to compile bindings; `post-build` copies curated `.d.ts` files and regenerates `build/llms.md`.
   - Keep `.d.ts` updates manual—`src/types/**/*.d.ts` are authoritative. Never auto-generate replacements.
   - Maintain parity between Rust structs/enums and their JS-facing equivalents. Update `postbuild-generate-docs.js` output when signatures shift.
3. **Verify**
   - Run targeted tests: `pnpm nx run compiler:test:rust`, `:test:js`, and `:test:typecheck`.
   - For project-aware workflows, clone fixtures under `libs/compiler/test/fixtures` instead of hitting external networks.
   - Confirm `Compiler.installSolcVersion` paths succeed when new solc ranges are introduced.

## Coding Standards

- Rust edition 2021, `cargo fmt` enforced.
- Use Biome for JS/TS formatting and linting (`pnpm nx run compiler:lint:js`).
- Maintain exhaustive error messages—surfaced through N-API as `Error` objects with actionable guidance.
- New features must include Bun tests or TypeScript type assertions inside `libs/compiler/test`.

## Helpful Commands

- Install deps: `pnpm install`
- Build: `pnpm nx run compiler:build`
- Post-build verification: `pnpm nx run compiler:post-build`
- Full test matrix: `pnpm nx run compiler:test`
- Update workspace graph: `pnpm nx graph --focus=compiler`

## Release Notes

- Production builds use `pnpm nx run compiler:build --configuration=production`.
- Always run `post-build` before publishing to ensure curated `.d.ts` and `llms.md` are in sync.
- Platform binaries live under `libs/compiler/build/npm/**`; keep them version-aligned with the Rust crate.

<!-- nx configuration end-->
