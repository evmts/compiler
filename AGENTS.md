<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

# Agent Onboarding

- **Project scope:** Maintain `@tevm/compiler`, a Rust + N-API compiler bridge consumed from Node.js/Bun. Core exports live in `libs/compiler/build/index.{js,d.ts}` after running the Nx build.
- **Workspace layout:** Use `pnpm nx graph --focus=compiler` to visualise dependencies. The only active project is `libs/compiler`, but tests depend on fixtures under `libs/compiler/test/fixtures`.
- **Environment:** Node 18+, pnpm 9+, Bun 1.1+, Rust stable. Solc binaries must exist in Foundry's `svm` cache (`Compiler.installSolcVersion`) before specs are executed. Vyper workflows require the `vyper` CLI on `PATH`.
- **Common commands:**
  - Install deps: `pnpm install`
  - Build bindings: `pnpm nx run compiler:build`
  - Post-build (copies `.d.ts`, type-checks, regenerates docs): `pnpm nx run compiler:post-build`
  - Test suites: `pnpm nx run compiler:test`, or the targeted variants `:test:rust`, `:test:js`, `:test:typecheck`
  - Formatting/linting: `pnpm nx run compiler:format`, `pnpm nx run compiler:lint`
  - Build, lint, test: `pnpm all`
- **Change workflow:** Prefer editing Rust + TypeScript in tandem so the generated `.d.ts` stays truthful. Every new feature needs regression coverage in Bun specs or TS type tests. Keep error messages descriptive—CLI users read them directly.
- **Gotchas:** Avoid editing files in `libs/compiler/build/**`; they are short-lived and auto-generated.

<!-- nx configuration end-->
