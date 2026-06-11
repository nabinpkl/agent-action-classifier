# ADR-0019: Policy lifecycle — compile once into an in-memory handle, decide many

Date: 2026-06-11
Status: Accepted

## Context
The schema prefactor ([ADR-0018](0018-validate-org-policy-against-cedar-schema.md)) made
`Policy::from_sources` — parse the Cedar schema, `Strict`-validate the policy set, parse the
entity store — run on **every** FFI call, pushing the Python round-trip from ~64µs to ~310µs.
That is author-time work landing on the per-decision hot path.

Research into how production policy engines handle this (OPA, AWS Verified Permissions, the
Cedar crate) returns one consistent model: policy exists in **three forms** and they are never
conflated.

1. **Authoring / source** — human-readable text (`.cedar`) in Git, reviewed and tested.
2. **Distribution / storage** — a packaged or managed artifact: an OPA *bundle* served to
   agents, or an AVP *policy store*. Cedar's own guidance is that `.cedar` text is for human
   display and JSON/protobuf is the canonical serialization.
3. **Evaluation / in-memory** — a parsed structure the engine runs against, built **once** and
   reused for many decisions ("parse once, evaluate many"; the Cedar crate ships
   `preparse_policy_set` / `stateful_is_authorized` for exactly this).

The pure core already gets this right: `Policy` *is* the in-memory compiled form, and the
conformance test builds it once and reuses it across all 9 cases. Only the binding conflated
forms 1 and 3 by re-compiling per call.

## Decision
1. **The in-memory compiled `Policy` is the evaluation form, and it crosses the FFI as a
   reusable handle.** The binding exposes `CompiledPolicy`: construction is the compile step
   (parse + `Strict`-validate the schema/policy/entities once, `ValueError` on drift), and
   `decide(action, context)` is the hot path with no policy parsing. Python wraps it as
   `Policy.compile(schema, policy, entities).decide(action, context)`.
2. **The host owns the handle.** A PEP (a per-agent hook) compiles its effective policy on load
   — or on a future plane push — and reuses it for every tool call. The held handle *is* the
   cache, mirroring an OPA agent holding its compiled bundle in memory.
3. **`.cedar` text stays the authoring source of truth in Git.** That is form 1 and it is
   already correct; this ADR only fixes how form 1 becomes form 3 at runtime.

## Consequences
- **Per-decision FFI drops back to the hot-path cost** (~310µs → ~20µs/call measured, compile
  excluded); the compile cost is paid once per policy load, where it belongs. The pure `decide`
  was always microsecond-class and is unchanged.
- **The core is untouched.** `decide(action, &Policy, context)` already took a pre-compiled
  handle, so the fix lived entirely in the binding + Python.
- **Breaking API change.** The Python surface moves from `decide(action, schema, policy,
  entities, context)` to `Policy.compile(...).decide(...)`. Acceptable at v0.
- **Reinforces [ADR-0018](0018-validate-org-policy-against-cedar-schema.md).** Schema validation
  still happens — it just moves to compile time, which is where fail-loud belongs (drift is
  caught when the policy loads, not per request).
- **Deferred, each with a revisit trigger:**
  - *Multi-policy / version cache* (a keyed cache of compiled handles) — trigger: the host holds
    more than one policy version at once. Until then the single held handle suffices (YAGNI; a
    keyed cache is the plane / bundle-server's concern, not the in-process agent's).
  - *JSON/protobuf storage + bundle/OPAL-style distribution* — trigger: the central plane
    persists and serves policies (also [ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md)
    deferred). The compiled-handle model is what those distribution forms hydrate into.
  - *A `version` id on the handle* (for audit and as a cache key) — trigger: the plane assigns
    policy versions.
