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
3. [x] **Org graph + inheritance resolution (ADR-0020).** Done as data, not engine code:
   Cedar evaluates hierarchy natively (`parents` + `in` + deny-overrides), so the pure core is
   untouched. `corpus/org_graph/` declares `Org<-Team<-User<-Agent` membership plus a
   cross-cutting `Role`, with node-attached + RBAC policies; 9 conformance cases prove cascade,
   sub-node override (same write Allowed for an eng agent, Denied for a sales agent), RBAC,
   team-scoped approval, and an org-wide hard deny — all 100% exact-match. Loader generalized to
   `load_corpus(name)`. Deferred-with-triggers: effective-entity slicing, per-agent policy
   slicing, multi-org, ReBAC, principal-side ABAC (ADR-0020).
4. [x] **Live per-agent hook PEP (ADR-0021).** Done as `enforce`, a Rust command-hook binary
   (`crates/policy_enforcement`) over the core: it normalizes a provider PreToolUse payload to
   the canonical action, `decide`s it against the org policy, and returns the verdict (allow=exit
   0, deny=exit 2 + reason, escalate=`permissionDecision:"ask"`). **One binary serves both Claude
   and Codex** (converged payloads); it **fails closed** on internal error (the binary owns its
   exit code, unlike Claude `type:http`). Chosen over the warm-handle HTTP sidecar (no daemon, one
   artifact) at ~5ms/call measured; the sidecar stays the documented latency roadmap. Reuses the
   asi05 plane via a new `resource_map.json` (glob->scope), so the demo needs zero new policy. v0
   governs mutation tools only. Probe-verified groundwork: a nested git repo (`experiments/`)
   isolates hooks for both providers, and `codex exec` fires no hooks (interactive only).
5. [x] **Decision-log record (ADR-0022).** Done in the `enforce` binary: given `--audit-log
   <path>` it appends one OPA/AAT-shaped JSON record per *governed* decision (request + verdict +
   gate_type + owasp + policy_id + lane + rationale + measured `latency_ns` + null `prev_hash`).
   The sink follows the PEP into Rust (ADR-0021) — `enforce` already holds the `Decision`, no host
   round-trip. **Fails closed** on a write failure (unrecordable decision is denied); out-of-scope
   calls leave no record. Deferred-with-triggers: the SHA-256 `prev_hash` chain + tamper-evident
   store, a central (vs per-agent) sink, per-call provenance id, rotation/retention.
6. [ ] **Semantic judge lane:** host LLM judge for `Escalate`, graded eval (80-90%, never
   exact-match).

## Later (deferred, with triggers in PRD/SPEC)

- [ ] Multi-tenant orgs (trigger: plane hosts >1 org).
- [ ] Policy distribution for multi-machine fleets (OPAL-style).
- [ ] Stateful/trajectory lane (ASI03/06/08).
- [ ] Hash-chained tamper-evident audit log.
