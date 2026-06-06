# ADR-0005: Organization-policy supremacy and standard authorization architecture

Date: 2026-06-06
Status: Accepted

## Context
The classifier governs agent actions against an organization policy. A core question
is the authority model: what happens when a local user approves an action the
organization would forbid. The product identity is "organization level": even if the
user approved, the organization policy has the final say. Separately, agent-action
governance is just authorization, a field with a mature, standardized architecture
(XACML's P*Ps) and a refined precedence model (AWS IAM), so it should be adopted
rather than reinvented (AGENTS.md: search the standard pattern first).

## Decision
**Adopt the standard authorization architecture and make organization policy supreme.**

- **Architecture = XACML P*Ps.** PDP (Policy Decision Point) is the pure,
  environment-independent decision core (Rust); PEP (Policy Enforcement Point) is the
  provider adapter that intercepts the action and enforces the verdict; PAP (Policy
  Administration Point) is the org policy file; PIP (Policy Information Point) supplies
  context (trajectory, scoped approval). This maps onto the dependency rule: pure core,
  impure edges.
- **Precedence = explicit-deny-overrides + default-fail-closed (AWS IAM model).**
  An *explicit* org deny is supreme and can never be overridden. An *implicit/default*
  deny can be lifted by an allow. Rules are combined with the **deny-overrides**
  algorithm (XACML-named) among the clauses *applicable* to the action.
- **Organization policy is supreme over user approval.** Org policy yields one of
  hard-deny (explicit deny) / hard-allow (explicit allow) / requires-approval (an
  implicit deny the org delegates). **Scoped** user approval only resolves the
  requires-approval case; it can never lift an explicit deny. Approval carries a scope
  (this action / class / session window) so a one-time consent cannot be replayed as
  blanket consent (itself ASI09, human-agent trust exploitation).
- **Default residue = escalate, then deny if unresolved.** A deliberate, justified
  divergence from authz's bare default-deny: a governance tool routes the ambiguous
  middle to a judge or human first, and only fails closed to deny if escalation does
  not resolve it (consent-based access control with a human-in-the-loop step).

## Consequences
- "Organization-level, even if the user approved" is now a precise, standard-backed
  rule, not a slogan.
- The judge (semantic lane) reasons *under* org supremacy: it decides whether an action
  violates org policy, with approval as a mitigating-not-overriding factor.
- The PDP-is-pure constraint is reinforced by the standard (PDP is environment-
  independent by definition), which is also what makes the core polyglot-embeddable.
