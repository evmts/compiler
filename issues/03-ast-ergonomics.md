# AST Ergonomics & Safety

## Scope
Targeted refactors in the `Ast` module to remove unsafe patterns, deduplicate logic, and clarify how AST parsing/stitching works. Medium effort with clear safety wins.

## Problems
- `expose_variables_internal` and `expose_functions_internal` use a raw pointer cast to work around borrow rules (`libs/compiler/src/ast.rs:146`), introducing unnecessary `unsafe`.
- Both visibility helpers duplicate iteration logic over `ContractDefinitionPart`, increasing maintenance cost (`libs/compiler/src/ast.rs:146`).
- Parsing and stitching helpers are tightly coupled to the NAPI-facing struct instead of living in a reusable service (`libs/compiler/src/ast.rs:207`, `libs/compiler/src/ast/parser.rs`).
- `Ast::sanitize_settings` overlaps with config sanitisation but lives in isolation, risking divergence (`libs/compiler/src/ast.rs:45` vs `libs/compiler/src/internal/config.rs:240`).
- `Ast::ast` only returns a JSON-compatible value, forcing Rust callers to re-deserialise when they already have the typed `SourceUnit` (`libs/compiler/src/ast.rs:242`).

## Proposed Direction
1. **Refactor visibility helpers** to stay within safe borrowing rules. Instead of storing a raw pointer, resolve contract indices first, then iterate with safe mutable references:
   ```rust
   fn mutate_contracts<F>(&mut self, overrides: Option<&AstOptions>, mut f: F) -> Result<()>
   where
     F: FnMut(&mut ContractDefinition),
   {
     self.update_options(overrides);
     let indices = {
       let unit = self.target_ast()?;
       self.contract_indices(unit, overrides)?
     };
     let unit = self.target_ast_mut()?;
     for idx in indices {
       if let SourceUnitPart::ContractDefinition(contract) = unit.nodes.get_mut(idx).ok_or_else(|| napi_error("Invalid contract index"))? {
         f(contract);
       }
     }
     Ok(())
   }

   fn expose_variables_internal(&mut self, overrides: Option<&AstOptions>) -> Result<()> {
     self.mutate_contracts(overrides, |contract| {
       for member in &mut contract.nodes {
         if let ContractDefinitionPart::VariableDeclaration(variable) = member {
           if matches!(variable.visibility, Visibility::Private | Visibility::Internal) {
             variable.visibility = Visibility::Public;
           }
         }
       }
     })
   }
   ```
   This removes the `unsafe` block currently at `libs/compiler/src/ast.rs:146`.
2. **Introduce a shared contract mutation helper** (as shown above) so both visibility functions and future mutations share the same traversal logic instead of duplicating loops (`libs/compiler/src/ast.rs:146`).
3. **Create an `AstService` module** responsible for parsing sources, stitching fragments, renumbering IDs, and sanitising values. The `Ast` struct just orchestrates config + state changes. For example:
   ```rust
   pub struct AstService;

   impl AstService {
     pub fn parse_source(source: &str, path: &str, solc: &Solc, settings: &Settings) -> Result<SourceUnit, AstError>;
     pub fn stitch_fragment(target: &mut SourceUnit, contract_idx: usize, fragment: ContractDefinition) -> Result<()>;
   }
   ```
   `Ast::from_source_string` and `Ast::inject_fragment_string` delegate to the service (`libs/compiler/src/ast.rs:207`).
4. **Reuse the central settings merger** (see config refactor) so `Ast::sanitize_settings` simply calls `settings::sanitize(settings)` and cannot diverge from compiler behaviour (`libs/compiler/src/ast.rs:45`).
5. **Expose typed AST accessors** such as:
   ```rust
   impl Ast {
     pub fn source_unit(&self) -> Option<&SourceUnit> {
       self.ast.as_ref()
     }
   }
   ```
   The NAPI `ast()` method continues returning a JSON-friendly shape but can be implemented in terms of `source_unit()` plus `AstService::sanitize_value`. This benefits Rust callers and tests (`libs/compiler/src/ast.rs:242`).

## Deliverables
- Safe visibility mutation helpers with shared iteration utilities.
- New service module housing parsing/stitching logic used by both AST and compiler flows.
- AST settings sanitisation wired to the shared settings merger.
- Additional accessor methods and updated NAPI bindings to keep the JS API stable.

## Side Effects & Risks
- Moving logic into services will shift existing unit tests; ensure coverage is preserved or expanded.
- Binding changes must maintain backwards compatibility for JS callers (`Ast::ast()` still returns the expected shape).
- Any new service module should remain `pub(crate)` to avoid expanding the public API prematurely.

## Validation
- Run existing AST Rust tests (`cargo test -p compiler -- ast::tests`) and Bun tests (`bun test libs/compiler/test/ast.spec.ts`).
- Add targeted tests for the new service layer to guarantee ID renumbering and stitching still behave correctly.
