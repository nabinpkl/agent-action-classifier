# ADR-0018: Validate the org policy against a Cedar schema (fail loud on drift)

Date: 2026-06-11
Status: Accepted

## Context
The Cedar swap ([ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md)) landed
with Cedar running **schema-less**: `Request::new(.., None)` and
`Entities::from_json_str(.., None)`. Cedar permits this, but without a schema it does no
validation of the policies or the entity store. A typo in an entity attribute
(`sensitvity` for `sensitivity`), a policy that reads an undeclared field
(`resource.classification`), or a wrong attribute type **silently produces a non-match** —
the request evaluates to a clean `Deny`/default-escalate instead of erroring. That is a
silent failure, which the project's first principle (AGENTS.md: *fail loud*) forbids.

The risk is small while the model is one hand-written `Agent` × `DataScope` corpus, but
TASKS #3 adds `Org`/`Team`/`Role`/`User` entity types, `parents` edges, and
`principal in Team::"…"` conditions. Every new type and attribute is another place a typo
becomes an invisible wrong answer. The cheap moment to close the trap is before the org
graph multiplies the surface.

Cedar already supports the fix: a schema validates the policy set (`Validator` under
`Strict` mode), the entity store (`Entities::from_json_str(.., Some(&schema))`), and the
request (`Request::new(.., Some(&schema))`). The schema is just another artifact the
central plane authors alongside the policies and entities.

## Decision
1. **The schema is part of the PAP.** The org policy is three Cedar artifacts authored
   together: the schema (the contract), the policies (the rules), and the entity store.
   `corpus/asi05/policy.cedarschema` is the corpus's contract; the host supplies its own.
2. **One validated construction path.** `Policy::from_sources(schema, policy, entities)` is
   the only constructor. It parses the schema, validates the policy set under
   `ValidationMode::Strict`, and parses the entity store against the schema, returning a
   typed `PolicyLoadError` (`Schema` / `Policies` / `Validation` / `Entities`) that names
   which artifact drifted. An in-memory `Policy` is therefore always schema-consistent.
3. **Requests are schema-checked too.** `evaluate` builds the Cedar request with
   `Some(policy.schema())`. A closed `ActionKind` maps only to schema-declared actions, so
   a valid `CanonicalAction` is schema-valid by construction (a build failure is a panic, a
   broken invariant, not a recoverable case).
4. **The wire carries the schema.** The FFI and the Python `decide` gain a `schema`
   argument (`decide(action, schema, policy, entities, context)`); validation failures
   surface to Python as `ValueError`.

## Consequences
- **Reinforces [ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md).** This is
  how the Cedar adoption keeps faith with fail-loud: drift between a policy and the entities
  it references now fails at load, at the edge where the operator can see it.
- **Every entity type and attribute must be declared.** Adding a data-scope attribute or a
  principal type is now a schema edit too. Fixtures must conform: the corpus and the FFI
  test entities carry the schema's full attribute set (e.g. `pii` on every `DataScope`).
- **Centralizes the loader dance.** The corpus loader and the Python wire previously each
  parsed policies + entities by hand; both now call `Policy::from_sources`, so the
  parse-and-validate knowledge lives in one place.
- **Adds per-call cost where the policy is parsed per call.** Schema parse + `Strict`
  validation runs on every `from_sources`, so the FFI round-trip rose (~64µs → ~310µs/call)
  because the binding still parses the policy per call. This is a host-caching problem, not
  an engine one: caching the parsed `Policy` keyed by the plane's policy version is the
  optimization (the pure `decide` is unchanged and still microsecond-class). Tracked, not
  yet done.
- **Bench stays schema-free.** `benches/cedar_decide.rs` measures raw `is_authorized`
  (schema validation is a load-time concern, off the timed path) over the #3 org-graph
  shape, which is ahead of the current schema. It is the speed gate, not the production
  loader, so it does not adopt the schema.
