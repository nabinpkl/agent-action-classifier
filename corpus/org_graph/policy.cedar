// Org-graph policy (ADR-0020): rules attached at nodes of the principal hierarchy. Each is
// scoped `principal in <Node>`, so it cascades to every agent transitively `in` that node.
// Precedence is Cedar's: a `forbid` on any node overrides a `permit` on an ancestor
// (deny-overrides), and an action matched by no rule defaults to escalate. All ASI05.

// Cascade: a permit on the whole org reaches every agent beneath it.
@id("org-allow-public-read")
@owasp("ASI05")
permit(principal in Org::"acme", action == Action::"read", resource)
when { resource.sensitivity == "public" };

// Org-wide permit to write confidential scopes — the baseline a sub-node can override.
@id("org-allow-confidential-write")
@owasp("ASI05")
permit(principal in Org::"acme", action == Action::"write", resource)
when { resource.sensitivity == "confidential" };

// Sub-node override: the sales team may NOT write confidential scopes. A forbid on the team
// node beats the org permit above (deny-overrides), so position in the graph changes the
// verdict for the same action.
@id("sales-forbid-customer-write")
@owasp("ASI05")
forbid(principal in Team::"sales", action == Action::"write", resource)
when { resource.sensitivity == "confidential" };

// RBAC: membership in the data-reader role grants reading confidential scopes, independent
// of which team the agent is in.
@id("reader-role-confidential-read")
@owasp("ASI05")
permit(principal in Role::"data-reader", action == Action::"read", resource)
when { resource.sensitivity == "confidential" };

// Team-scoped approval: the eng team may write restricted scopes, but only with a scoped
// user approval (checked host-side). Sales is not in eng, so this rule never applies to it.
@id("eng-approve-config-write")
@owasp("ASI05")
@outcome("requires_approval")
@lane("deterministic")
permit(principal in Team::"eng", action == Action::"write", resource)
when { resource.sensitivity == "restricted" };

// Org-wide hard deny: nobody under acme may write a secret-classified scope. Supreme,
// unoverridable by approval.
@id("org-forbid-secret-write")
@owasp("ASI05")
forbid(principal in Org::"acme", action == Action::"write", resource)
when { resource.sensitivity == "secret" };
