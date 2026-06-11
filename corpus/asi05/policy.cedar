// ASI05 (Unsafe Code Execution) org policy, expressed as Cedar over the data-scope
// model (ADR-0017). Each rule carries the annotations the host reads to reconstruct the
// four-verdict cascade: @id (stable audit id), @owasp (clause), and on permits @outcome
// + @lane. A `forbid` is a hard deny; deny-overrides and default-deny are Cedar's.

// Hard deny: writing a secret-classified scope is supreme, unoverridable by approval.
@id("deny-secret-write")
@owasp("ASI05")
forbid(principal, action == Action::"write", resource)
when { resource.sensitivity == "secret" };

// Hard allow: reading a public scope is permitted deterministically. No gate annotation:
// a plain permit with no @outcome/@lane is a terminal hard allow.
@id("allow-public-read")
@owasp("ASI05")
permit(principal, action == Action::"read", resource)
when { resource.sensitivity == "public" };

// Requires approval: writing a restricted scope is an implicit deny the org delegates to
// a scoped user approval (checked host-side, not in Cedar).
@id("approve-restricted-write")
@owasp("ASI05")
@outcome("requires_approval")
@lane("deterministic")
permit(principal, action == Action::"write", resource)
when { resource.sensitivity == "restricted" };

// Semantic: reading PII needs the judge's reasoning; this permit routes to the semantic
// lane and shadows any co-applicable hard allow.
@id("judge-pii-read")
@owasp("ASI05")
@lane("semantic")
permit(principal, action == Action::"read", resource)
when { resource.pii == true };
