# TASKS.md

Current iteration task list for `agent-action-classifier`.

## Done

- [x] Define the initial PRD scope. (PRD.md, Accepted 2026-06-06)
- [x] Choose and document the technical stack in SPEC.md.
- [x] Scaffold the Rust PDP core + Python host skeleton + `just check` gate.
- [x] First vertical slice: pure `decide` + ASI05 deterministic conformance corpus (9 cases).

## Re-founding (ADR-0017: Cedar engine + org-first central plane)

The hand-built engine and the placeholder `Operation` enum are superseded. The corpus
externalization (ADR-0010) and the PyO3 host/binding split (ADR-0011) survive as the host
boundary; the matcher/precedence core is replaced by Cedar. Priority order:

1. [x] **Cedar speed spike (the gate).** Done: `benches/cedar_decide.rs` (cedar-policy
   4.11, dev-dep). **PASS.** Embedded `is_authorized` over the org-first shape: ~4µs @ 5
   policies, ~15µs @ 25, ~58µs @ 100; **flat with entity count** (~15.5µs @ 16 vs 150
   entities, indexed lookup). At a representative ≤25-policy node the eval is ~15µs, ~6.5x
   under the 100µs budget and in line with Cedar's published single-digit-µs (ref Microsoft
   <0.1ms, ADR-0006). Policy count is the scaling axis; inheritance + Cedar slicing keep the
   per-request set small. Gate met -> proceed to the core swap.
2. [x] **Swap the core to Cedar.** Done: `Matcher`/precedence and the placeholder
   `Operation` enum are gone. `decide` builds a Cedar request, runs `is_authorized`, and
   reconstructs the four-verdict cascade host-side from the determining policies'
   annotations (`@id`/`@owasp`/`@outcome`/`@lane`); a `forbid` is the supreme hard deny.
   Corpus pivoted to the data-scope model (`policy.cedar` + `entities.json` + `cases.json`),
   all 9 cases pass at 100% exact-match through Cedar. Wire + Python carry Cedar source +
   entity JSON. `cedar-policy` is now a real dep. FFI round-trip ~64µs/call incl. per-call
   Cedar parse (host caching is the optimization).
   - [x] **Hardening prefactor (ADR-0018): schema-validate the org policy.** Added
     `policy.cedarschema` as the contract; `Policy::from_sources` is the one path, validating
     policies under Cedar `Strict` and parsing entities against the schema (typed
     `PolicyLoadError`). Requests are schema-checked. Drift now fails loud at load instead of
     silently non-matching, before #3 multiplies entity types. Wire + `decide` gained a
     `schema` arg; FFI round-trip rose to ~310µs/call from the per-call schema validation
     (host caching of the parsed `Policy` is the tracked optimization).
   - [x] **Hardening prefactor (ADR-0019): parse-once compiled handle.** Split the FFI into
     compile vs decide, the industry policy lifecycle (OPA bundles / Cedar preparse / AVP
     store): `CompiledPolicy` parses + validates once, `.decide(action, context)` is the hot
     path. Python API is `Policy.compile(schema, policy, entities).decide(action, context)`.
     Per-decision FFI back to ~20µs/call (compile paid once per load); the pure core was
     already split, so the fix was binding-only.
3. [~] **Org graph + inheritance resolution (ADR-0020) — built, then removed in v1 (ADR-0025).**
   Originally done as data (Cedar evaluates `parents` + `in` + deny-overrides natively): a
   `corpus/org_graph/` plane with `Org<-Team<-User<-Agent` + `Role`, 9 cases proving cascade,
   sub-node override, RBAC, team-scoped approval, and an org-wide hard deny at 100% exact-match.
   But the plane was wired to nothing live (the PEP loads the flat `asi05`), and v1 is a flat
   single principal, so the hierarchy was speculative generality in a fixture. **Removed** the
   plane + its conformance test (ADR-0025); `load_corpus(name)` stays generic as the seam a future
   plane plugs into. Re-introduce when a real multi-agent org exists (trigger in ADR-0025).
4. [x] **Live per-agent hook PEP (ADR-0021).** Done as `enforce`, a Rust command-hook binary
   (`crates/policy_enforcement`) over the core: it normalizes a provider PreToolUse payload to
   the canonical action, `decide`s it against the org policy, and returns the verdict (allow=exit
   0, deny=exit 2 + reason, escalate=`permissionDecision:"ask"`). **One binary serves both Claude
   and Codex** (converged payloads); it **fails closed** on internal error (the binary owns its
   exit code, unlike Claude `type:http`). Chosen over the warm-handle HTTP sidecar (no daemon, one
   artifact) at ~5ms/call measured; the sidecar stays the documented latency roadmap. Reuses the
   asi05 plane via a new `resource_map.json` (glob->scope), so the demo needs zero new policy. v0
   governs mutation tools (extended to shell commands in #6). Probe-verified groundwork: a nested
   git repo (`experiments/`) isolates hooks for both providers, and `codex exec` fires no hooks
   (interactive only).
5. [x] **Decision-log record (ADR-0022).** Done in the `enforce` binary: given `--audit-log
   <path>` it appends one OPA/AAT-shaped JSON record per *governed* decision (request + verdict +
   gate_type + owasp + policy_id + lane + rationale + measured `latency_ns` + null `prev_hash`).
   The sink follows the PEP into Rust (ADR-0021) — `enforce` already holds the `Decision`, no host
   round-trip. **Fails closed** on a write failure (unrecordable decision is denied); out-of-scope
   calls leave no record. Deferred-with-triggers: the SHA-256 `prev_hash` chain + tamper-evident
   store, a central (vs per-agent) sink, per-call provenance id, rotation/retention.
6. [x] **Shell-command lane (ADR-0023).** `enforce` now governs Bash (-> `execute`): the host
   classifies the command line into a stable kind via operator-tunable `command_signatures.json`
   (regex -> kind), carried into the Cedar request as `context.command.kind`; the rules decide on
   the classification, never the raw string. Three kinds, all `requires_approval` (escalate ->
   ask) with distinct `@id`: `package_install` (npm/yarn/pnpm/bun/uv/pip/cargo/brew/gem/go +
   system), `ephemeral_exec` (npx/dlx/bunx/uvx/pipx run/`go run pkg@v`/deno run/gem exec),
   `pipe_to_shell` (curl|sh and kin). Opens the `CedarContext::empty()` seam: `CommandFacts` on
   the canonical action maps to context at the engine edge. Fails closed on a Bash call wired
   without `--command-signatures`. Brittleness ceiling (obfuscation, split download-then-run)
   stays the ADR-0003/trajectory gap. `pipe_to_shell` is the first candidate to graduate to a
   hard forbid.
7. [ ] **Semantic judge lane:** host LLM judge for `Escalate`, graded eval (80-90%, never
   exact-match).

## Later (deferred, with triggers in PRD/SPEC)

- [ ] Multi-tenant orgs (trigger: plane hosts >1 org).
- [ ] Policy distribution for multi-machine fleets (OPAL-style).
- [ ] Stateful/trajectory lane (ASI03/06/08).
- [ ] Hash-chained tamper-evident audit log.
