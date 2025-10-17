# Housekeeping & Build Hygiene

## Scope

Small-but-impactful cleanups that reduce build noise, shrink the repository, and keep generated artifacts consistent.

## Tasks

- **Remove unused proc-macro dependencies** (`syn`, `quote`, `proc-macro2`) from `libs/compiler/Cargo.toml:19`. Verify the crate compiles without them; they appear to be remnants from earlier macro experiments.

  ```toml
  # before
  syn = { version = "2.0", features = ["full", "visit"] }
  quote = "1.0"
  proc-macro2 = "1.0"

  # after (delete)
  ```

- **Stop committing platform-specific `.node` binaries** like `libs/compiler/compiler.darwin-arm64.node`. Instead, add them to `.gitignore` and teach CI or release scripts to build/upload binaries. This keeps the repo lean and avoids stale binaries.
- **Clarify the manual `copy-types` script** workflow in repo docs. We intentionally copy certain TypeScript typings authored in JavaScript because they cannot currently be derived from the Rust bindings via `napi`. Add a short comment or README note outlining this rationale so future housekeeping passes do not attempt to automate it away.
- **Refresh documentation** in `libs/compiler/README.md` and `libs/compiler/src/ast/README.md` once the refactors land. Document the new module layout, the core Rust API, and how JS consumers should interact with the bindings. Include an outline of the compilation pipeline and AST workflow so newcomers can navigate quickly.

## Validation

- Run `cargo metadata` / `cargo check` to confirm dependency removal does not break builds.
- Execute `npx nx run compiler:build` to ensure the new type generation flow produces expected outputs and that binaries are handled externally.
- Share README updates with the team for review to keep documentation aligned with the new architecture.
