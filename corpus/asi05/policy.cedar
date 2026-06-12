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
// that classification, never on the raw command string. All three require approval in v0
// (escalate -> ask): a shell command of these kinds is surfaced to a human, who decides. Each
// guards `context has command` so an unclassified execute simply matches nothing.

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
// arbitrary remote code with no registry or provenance. The first candidate to graduate from
// ask to a hard forbid.
@id("approve-pipe-to-shell")
@owasp("ASI05")
@outcome("requires_approval")
@lane("deterministic")
permit(principal, action == Action::"execute", resource)
when { context has command && context.command.kind == "pipe_to_shell" };
