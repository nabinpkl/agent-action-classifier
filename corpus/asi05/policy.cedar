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

// --- Shell-command governance (ADR-0023) -------------------------------------------------
// The PEP classifies a Bash command into a stable `context.command.kind`; these rules gate on
// that classification, never on the raw command string. Two kinds require approval (escalate
// -> ask): the command is surfaced to a human, who decides. The third (pipe-to-shell) is a hard
// forbid -- approval cannot make unknown remote bytes safe (ADR-0026). Each rule guards
// `context has command` so an unclassified execute simply matches nothing.

// Installing a dependency or tool (npm/yarn/pnpm/bun/uv/pip/cargo/brew/gem/go ... add|install)
// runs install-time/build scripts = code execution, even on a plain lockfile restore.
@id("approve-package-install")
@owasp("ASI05")
@outcome("requires_approval")
@lane("deterministic")
permit(principal, action == Action::"execute", resource)
when { context has command && context.command.kind == "package_install" };

// Ephemeral fetch-and-run (npx / pnpm|yarn dlx / bunx / uvx / pipx run / go run pkg@v / deno
// run / gem exec): downloads and executes a package without a persistent install.
@id("approve-ephemeral-exec")
@owasp("ASI05")
@outcome("requires_approval")
@lane("deterministic")
permit(principal, action == Action::"execute", resource)
when { context has command && context.command.kind == "ephemeral_exec" };

// Piping a fetched script into an interpreter (curl|sh, wget|bash, sh -c "$(curl ...)"):
// arbitrary remote code with no registry, pin, or provenance -- and approval cannot make it
// safe, because the bytes are unknown until they already run. So this is a hard forbid (deny,
// unoverridable by approval, cross-provider), the first command kind to graduate from ask
// (ADR-0026). Like deny-secret-write, it carries no @outcome/@lane: a forbid is a terminal deny.
@id("forbid-pipe-to-shell")
@owasp("ASI05")
forbid(principal, action == Action::"execute", resource)
when { context has command && context.command.kind == "pipe_to_shell" };
