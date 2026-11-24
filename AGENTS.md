<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

# Agent Onboarding

- **Project scope:** Maintain `@tevm/compiler`, a Rust + N-API compiler bridge consumed from Node.js/Bun. Core exports live in `libs/compiler/build/index.{js,d.ts}` after running the Nx build.
- **Workspace layout:** Use `pnpm nx graph --focus=compiler` to visualise dependencies. The only active project is `libs/compiler`, but tests depend on fixtures under `libs/compiler/test/fixtures`.
- **Environment:** CI uses Node 22, pnpm 10.28.2, Bun 1.4.1, and Rust stable. Install solc 0.8.29 and 0.8.30 in the `svm` cache and put Vyper 0.4.3 on `PATH` before running the full suite.
- **Common commands:**
  - Install deps: `pnpm install --frozen-lockfile` and `pnpm --dir libs/compiler install --frozen-lockfile`
  - Build bindings: `pnpm nx run compiler:build`
  - Post-build (copies `.d.ts`, type-checks, regenerates docs): `pnpm nx run compiler:post-build`
  - Test suites: `pnpm nx run compiler:test`, or the targeted variants `:test:rust`, `:test:js`, `:test:typecheck`, `:test:e2e`
  - Benchmarks: `pnpm bench`
  - Formatting/linting: `pnpm nx run compiler:format`, `pnpm nx run compiler:lint`
  - Build, lint, test: `pnpm all`
- **Change workflow:** Prefer editing Rust + TypeScript in tandem so the generated `.d.ts` stays truthful. Every new feature needs regression coverage in Bun specs or TS type tests. Keep error messages descriptive—CLI users read them directly.
- **Gotchas:** Avoid editing files in `libs/compiler/build/**`; they are short-lived and auto-generated.

<!-- nx configuration end-->
