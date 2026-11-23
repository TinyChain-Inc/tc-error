# tc-error invariants

- Own shared error kinds, diagnostic context, structured pressure data, and
  format-neutral codecs. Keep transport formatting and lifecycle policy in callers.
- Preserve locally established recovery provenance through conversions, context,
  and cloning. Never serialize it or derive it from remote error messages.
- Delegate classification through native variants and retained causes. Do not match
  diagnostic strings or add storage recovery logic here.
- Keep adapters optional and independently compilable. Features add conversions,
  not alternate error models or wire representations.
- Keep Rust safe and retain `#![forbid(unsafe_code)]`.
- Borrow for inspection; move consumed causes. Format diagnostics at the reporting
  boundary and retain shared semantics in one implementation.
- Group Rust imports as standard library, dependencies, then local modules.
  Keep related declarations together and separate logical phases and multiline
  statements with a blank line. Follow rustfmt and strict Clippy.
- Test observable classification, codecs, context preservation, and local recovery
  provenance. Reuse cases; avoid source-pattern checks, prose assertions, duplicate
  adapter permutations, and tests of test fixtures.
- Keep required behavioral and workflow guidance independently usable here.
  Parent-workspace documents are not dependencies.
