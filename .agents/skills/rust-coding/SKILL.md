---
name: rust-coding
description: Idiomatic Rust for the PDP core, thiserror for the library / anyhow for glue, clippy -D warnings, borrow-over-clone, newtypes, closed enums with exhaustive matches. Use when writing or reviewing Rust in this repo.
---

# Rust coding (the PDP core)

The PDP is a **library crate**, so its conventions are library conventions. Follow
[AGENTS.md](../../../AGENTS.md) first: fail loud, custom error types at boundaries,
borrow over clone, no silent fallbacks. Don't write C++-in-disguise; lean on the type
system.

Sources: [Effective Rust: error types](https://lurklurk.org/effective-rust/errors.html),
[thiserror vs anyhow](https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling),
[Rust 2026 practices](https://dasroot.net/posts/2026/04/rust-2026-new-features-best-practices/).

## Error handling

- **The PDP core uses `thiserror`.** It's a library; callers must branch on failure
  modes, so derive `thiserror::Error` on a typed enum, with `#[from]` for transparent
  wrapping and `#[source]` for chains. This is AGENTS.md's "custom error types at
  library boundaries" made concrete.
- **`anyhow` only for application/glue code** (benches, the host-side Rust, examples)
  where the error gets logged or displayed, not matched. Add `.with_context(|| ...)?` at
  boundaries so the chain reads like a story.
- **Reserve `panic!`/`unwrap`/`expect` for broken invariants**, never for recoverable
  errors. No `unwrap_or_default()` to paper over a failure, propagate the `Result`.

## Types and ownership

- **Newtype the ids.** `AgentId`, `SessionId`, `RuleId`, `OwaspClause` (already in
  [SPEC.md](../../../SPEC.md)) are newtypes, not bare `String`, so the API can't confuse
  them and the compiler enforces intent.
- **Borrow over clone.** `decide(&CanonicalAction, &Policy, &Context)`; cloning in the
  hot path defeats the zero-allocation matching goal. Clone only when ownership is truly
  needed.
- **Closed sets are enums; matches are exhaustive.** The `Operation` variant set and the
  verdict/outcome types are enums. **No catch-all `_` arm** that would silently swallow a
  new variant, let the compiler force a decision when a variant is added (this is the
  fail-loud rule at the type level).
- Keep the `pub` surface small and literal; the deep module exposes one `decide` and
  hides the engine. Derive standard traits (`Debug`, `Clone`, `PartialEq`) where cheap.

## Lint and format

- CI runs `cargo clippy --all-targets --all-features -- -D warnings` and
  `cargo fmt --check`. **No blanket `#[allow]` at the crate root**, only narrow,
  commented allows that explain why.
- Benchmark on `--release`; never quote debug-build numbers (ties to the
  reference-or-frontier measurement rule).
