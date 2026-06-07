# AGENTS.md

Conventions and principles for this project. Keep entries short. General principles first; language-specific rules belong at the bottom only after the stack is chosen.

## Do's and Don'ts (hard rules — re-read before each task)

These counter known agent tendencies. Re-read this list when deep in a long task; older instructions get out-attended by recent context (goal drift is real).

**Do**
- Fail loud: surface every error. Return/propagate it or panic on a broken invariant. → `load_file(p)?` or `return Err(LoadError::NotFound(p))`, never `Ok(String::new())` on a missing file.
- At v0, break freely: delete and rewrite. → rename `Buffer` → `TextBuffer` everywhere in one pass; don't keep a `Buffer` alias around.
- Grep for the concept before creating any file/folder. → run `grep -rli "undo"` first; if `history/` already exists, add redo there, don't make `undo_history/` beside it.
- Re-read these rules before large edits; don't trust the copy in your head from 50k tokens ago. → before a multi-file refactor, re-open this file.
- Plan the change before writing the first line. → list the files you'll touch and the order, then code.
- Push back on a wrong instruction. → "You asked to catch and ignore the save error, but that hides data loss — propagate it instead?" rather than silently doing it.
- Keep the change runnable AND correct. → code that compiles but returns the wrong classification isn't done; add the test that proves it.
- Check actual state before acting; never trust a cached assumption. → run `git status` / `ls` / `cat` and verify, e.g. don't `git init` a repo that's already initialized.
- Assume the human is editing alongside you (copilot mode, shared codebase). → re-read a file right before you change it; your in-context copy may be stale. Don't clobber their unsaved/just-made edits.
- Search the standard pattern first. → before a major feature, web-search the idiomatic approach with the year and build on it; don't roll it from stale model memory.
- Treat these rules as mutable working agreements. → if a rule blocks the work or has gone stale, propose the AGENTS.md edit in the same change; never silently work around it or defend it with "AGENTS.md says so". The code is the source of truth; this file is meta-guidance over it.
- Surface a rule-vs-code contradiction; don't silently obey the rule. → if this file says one thing and the code does another, flag the mismatch and say which side should change — don't treat AGENTS.md as authoritative over reality.
- Surface architectural drift instead of building on the wrong layer. → when a feature means stacking on a primitive that no longer fits, say "this layer is fighting us, here's a cleaner shape and the tradeoffs" rather than hard-layering on top. Proposing a big refactor is the job, not overstepping.
- Write tests only for desired behavior of the current architecture; do not write tests that assert the absence of previous architectural patterns; add/refer to ADRs for historical context.”


**Don't**
- Don't add silent fallbacks, placeholder/default-on-error data, or error-swallowing. → no `load_file(p).unwrap_or_default()`, no `let _ = risky();` to mute a warning, no empty `match ... { Err(_) => {} }`. Let the `Err` propagate.
- Don't add backward-compat shims, versioning, or deprecation paths at v0. → don't keep `fn save_old()` next to `fn save()` or a `#[deprecated]` wrapper; just change `save()`.
- Don't over-engineer: no speculative generality, no abstraction for a future that isn't here (AHA + Rule of Three). → don't add `trait Storage` with one `DiskStorage` impl "in case we add cloud later"; use the concrete type until a second impl actually exists.
- Don't create `utils`/`helpers`/`core`/`manager` dumping grounds, or near-duplicate concept names. → no `utils.rs`; no `file_writer.rs` next to `save_file.rs` (same concept, pick one).
- Don't let output drift from the request. → asked to "add redo" but you find yourself rewriting the whole undo stack — stop and confirm before continuing.
- Don't claim done on partial work. → if you skipped a case or a test fails, say so with the output; don't paper over it to make something "pass".
- Don't leave dead code. → removed means deleted entirely (file, imports, types, references). No commented-out "just in case" blocks — git remembers.
- Don't build god files. → one file doing parsing + state + IO gets split by concern, not left to grow into a 1000-line file.
- Don't hardcode tunable values. → model names, thresholds, paths, and prompts go through config, not literals sprinkled across source. Protocol constants stay inline.
- Don't add toggle/mode flags that change trust or data semantics. → different behavior comes from a different entry point or type, not a `if v2 { ... }` switch.



## Project Docs (read before project work)

AGENTS.md is **how to operate** in this repo. Product intent and technical shape
live in sibling docs — read them before any project work, and keep their content
there, not duplicated here:

- **[PRD.md](PRD.md)** — product: what we build, why, scope, non-goals, deferred. (the "what" / "why")
- **[SPEC.md](SPEC.md)** — technical shape: stack, module boundaries, contracts, budgets. (the "how")
- **[TASKS.md](TASKS.md)** — the current iteration's task list.
- **[docs/adr/](docs/adr/)** — Architecture Decision Records: one load-bearing decision per file.

Scope guard: before adding a feature, check `PRD.md` scope. If it's out of scope
and needs a breaking architectural change, stop — defer it or reopen scope with the
user. Reopening scope is a deliberate decision, not a refactor.

## How to Explore This Codebase (do this first)

There is no static map to read or maintain — derive it live from the filesystem.
The folder/file names should be literal concepts, so the tree alone tells you
where things are. Explore progressively, widening only as needed:

1. **See the shape (shallow):**
   `tree -L 2 .` (or `find . -maxdepth 2 -type f` if `tree` is missing).
   File names are the concepts. You can usually guess the target from this alone.
2. **Find a concept by name:** `grep -rln "classifier" .` → the file should be named after that concept. Search the literal word you'd use for the feature.
3. **Read a concept's public surface first:** open its file or module entry. Drill into the body only when you need the internals.
4. **Trace a flow:** grep for the function, type, or command name to jump across boundaries by signature.

Rule: never assume a static description of the code; run the command, read what's
actually there. The filesystem is the source of truth and never drifts.

## General Principles

### Agent-Legible & Grep-Friendly (read this first)
- **Name things the literal thing they are.** A name is an agent's primary signal — it acts on the name without reading the body. `ActionClassifier`, `PromptExample`, `DecisionLog` — never `core`, `utils`, `helpers`, `manager`, `service`, `handler`.
- **Write for grep, not for cleverness.** If you'd search `classify` to find it, name it `classify_action`. Avoid runtime registries, dynamic dispatch, and clever indirection — grep can't see them; an explicit name it can.
- **The guess test:** you should know what's in a file from its name alone, without opening it. If you have to open it to learn what it does, rename it.
- **Vertical slice:** put everything for one concept together, named after the concept. Domain-driven by literal names, not by old-DDD layer roles.
- **The structure IS the map.** No `CODEMAP.md`, no maintained index — they rot. The live filesystem, with literal names, is the always-fresh map. Explore it on demand.

### Search Before You Add (no duplicate concepts)
- **Before creating any file or folder, grep for the concept first.** `grep -rli "classify"` / `find . -iname "*classify*"`. If it exists, extend it — don't make a sibling.
- **Ban near-duplicate names.** No `action_history/` next to `history/`, no `save_file.rs` next to `file_writer.rs`. Same concept → one home, one name. Match the meaning, not just the spelling.
- If a concept seems new, check it isn't an existing one renamed. The literal-naming rule makes this cheap — search the word you'd use for the feature.

### Duplication: AHA + Rule of Three
- **DRY is about knowledge, not text.** Deduplicate only when two pieces encode the *same decision*; code that merely looks alike but changes for different reasons stays separate.
- **Rule of Three:** first time write it, second time tolerate the copy, third time abstract. Abstracting at two is premature.
- **AHA — Avoid Hasty Abstractions.** A wrong abstraction is costlier than duplication. Prefer a little copy-paste over the wrong shared helper. Never create `utils`/`helpers` to hold "shared" code.
- Agents silently duplicate (they generate whether or not it exists) — that's why Search-Before-You-Add above is mandatory.

### File Size & Splitting
- **No magic number; ~500 lines is a smell, not a limit.** Split only when a self-contained cluster has formed its own identity inside the file.
- Split at natural seams (one concept extracts cleanly), never arbitrarily mid-concept. Keep things you read together in one place (locality > dogmatic DRY).
- After a split, each new file's name must self-describe its slice. If you can't name the split cleanly, it isn't ready to split.

### Cohesion & Coupling
- Aim for **high cohesion** (things in a module share one purpose) and **loose coupling** (modules depend on each other minimally, only through stable contracts).
- Test for "same module?": do these change for the same reason? Yes → together. No → split.

### Separation of Concerns
- Each part should change for exactly one reason. Don't mix I/O, business logic, and presentation in one place.

### Single Source of Truth
- Make a feature work off one authoritative piece of state first. If you can't and must copy or cache it, surface that to the user before duplicating — drift between copies is a top bug source.

### Config vs Constants
- **Config** = values a user or environment would tune: model names, thresholds, paths, prompts, keybindings. Centralize in `config`; don't scatter literals.
- **Constant** = protocol/internal values nobody should change: framing bytes, regex anchors, parser sentinels. Leave inline where they're used.
- Migration trigger is the *second* consumer: one literal in one function is fine; the moment a second place needs the same value, move it to `config` in the same change.

### No God Modules
- No god file, module, or function. If one file mixes more than one concern (parse + state + IO), extract — composition of small literal-named pieces, not one growing blob.
- Extract when extraction earns its place, not before (pairs with AHA + Rule of Three).

### Dependency Rule (most important)
- Source-code dependencies point **inward, toward the domain logic**. Pure logic (classification, scoring, parsing) never depends on infrastructure (disk, network, LLM APIs, CLI).
- Invert the dependency: the logic defines what it needs; the real-world side implements it. The pure side is testable with fakes, zero infrastructure.
- Don't name folders after this role (no `core/`, `ports/`, `adapters/`) — name them the literal concept. The dependency direction lives in the imports, not the folder names.

### Package by Concept, not by Layer
- Top-level folders are the literal concepts of the program, not framework roles (`controllers/`, `models/`) and not abstract roles (`core/`, `services/`).
- Everything for one concept lives together. Light internal structure inside a concept is fine.

### Abstract Only at Boundaries
- Introduce an interface/port **only** for things touching the real world (filesystem, terminal, network, clock, DB, LLM APIs) or for genuinely complex logic.
- Plain internal logic stays concrete. Over-abstraction is indirection tax (YAGNI).

### Existing Code Is Not Sacred
- Existing code is iteration in progress, not a spec to defend. "It matches what we have" and "it minimizes churn" are anti-reasons when the current shape is the problem.
- Don't defend the codebase as architecturally sound just because it exists. When you see an ad-hoc pattern, a wrong primitive, or a layer fighting you, flag it and propose the cleanup in the same change.
- **Surface architectural drift proactively.** Simple architecture is correct at the start; as features accrete there is a point where one more feature would stack on a primitive that no longer fits. When you feel "this is getting hard to navigate / this primitive is fighting us," say so — don't quietly pile on.
- **Don't be afraid to propose a large refactor or re-architecture.** Bring the tradeoffs and the payoff: "moving X to Y costs this, but makes Z and the next features easier." Surfacing it is your job; the human decides whether to take it.
- Re-found the layer when it stops fitting rather than hard-layering on top. Better to clean as the shape forms than to keep building on the wrong foundation.

### Functions
- Small, single-purpose, one level of abstraction each.
- Separate construction from use: pass collaborators in, don't build them inside (dependency injection → testable).

### Naming
- Names describe intent, not mechanism (`cursor_position`, not `cp`).
- Consistency beats correctness: one pattern applied everywhere.
- Name length scales with scope. Loop `i` is fine; an export needs a full name.

### Error Handling
- Custom error types at library boundaries so callers can branch on failure modes.
- Preserve the error chain (don't lose the root cause).
- Reserve panics/aborts for truly unrecoverable invariants.

### Testing
- Testing pyramid: many unit tests, fewer integration, few end-to-end.
- Colocate unit tests with the code; integration tests separate.

### Process
- Keep each change small and single-purpose.
- Build, test, lint, and format locally before committing.
- Use a `justfile` for common workflows once the stack is chosen: `just check` (build + test + lint + fmt), `just dev` (run the app/tool). One named surface — don't duplicate recipes or hand-type the raw commands each time.
- Review for behavior, edge cases, security, performance — not just style.
- KISS, YAGNI, DRY, SOLID are tactics serving cohesion/coupling — apply, don't worship.

### Dependency / Library Bar
Before adding any dependency, check the bar — a bad dep is a long-term liability:
- Actively maintained: a recent release and human-merged PRs, not just `dependabot`/`renovate` bumps.
- Enough adoption that bugs are shaken out; clean README, no "unmaintained" banner.
- Prefer the latest stable release. Don't rebuild what a maintained package already does well.
- If a dep fails the bar but nothing better exists, record the accepted risk and a revisit-trigger as an ADR under `docs/adr/`.

### Architecture Decisions (ADRs)
- Record each load-bearing architectural, design, or organizational decision made *outside explicit direction* as an ADR in `docs/adr/` (`NNNN-title.md`). See [docs/adr/0000-record-architecture-decisions.md](docs/adr/0000-record-architecture-decisions.md).
- Each ADR: context, the decision, consequences — with standards-backed reasoning independent of any one prompt. Dated; immutable once Accepted — supersede with a new ADR rather than editing.
- Reserve ADRs for load-bearing choices; obvious or local decisions don't need one.

### Commit Hygiene (agentic era)
- **Atomic commits:** one logical change each. Split sweeping agent output into focused commits — keeps review, `bisect`, and rollback sane.
- **Subject = Conventional Commits**, body = the "why". Format: `type(scope): summary` (imperative, ≤72 chars, lowercase, no trailing period).
  - Types: `feat` (new capability), `fix` (bug), `refactor` (no behavior change), `test`, `docs`, `chore` (tooling/deps), `perf`, `style` (formatting), `build`, `ci`, `revert`.
  - Scope = the literal concept touched: `feat(action_classifier): add correction detection`, `fix(decision_log): handle missing file on load`.
  - Breaking change → `feat(action_classifier)!: ...` or a `BREAKING CHANGE:` footer.
- **Body captures the Decision Shadow:** why this change, constraints, rejected alternatives — the reasoning the diff can't show. Note what you verified ("tested manually with valid + invalid input").
- **Provenance in trailers, never the subject.** Attribution must be accurate, granular, and intentional — never auto-blanket. A wrong/false trailer is worse than none. This repo's `Co-Authored-By` trailer is the consensual, intentional kind — keep it.
- **Explore-many, commit-one:** scratch/exploration paths stay out of main history; land only the cleaned, chosen path.
- **Commit at the arc boundary, not the moment work looks done.** The commit unit is a logical *arc* (one fix, or one feature), not a turn or an instruction. Hold the change in the working tree; don't fire the instant it compiles — an immediate commit bakes in agentic bugs before any human has reacted.
  - **An arc spans as many turns as it needs:** the opening instruction plus the corrections that settle it. It stays uncommitted while open.
  - **On each new human instruction, classify it.** *Correction/refinement of an open arc* ("that's wrong", "also handle X there") → don't commit; fold it in, the arc stays open. *Forward/distinct work* → the arc the human just moved on from has cleared the strongest signal available (they saw enough to leave it); if it's coherent and the pre-commit hook passes, **autocommit + push that arc alone**, then start the new one.
  - **One instruction can span multiple arcs** ("fix X and start Y") → split into separate atomic commits; never merge a fix and a feature. **Multiple arcs can close in one turn** → multiple commits land that turn.
  - **Never commit half-done or hook-failing work**, even at a boundary — commit only the part that stands alone, or keep waiting. When unsure whether an instruction touches an open arc, **bias to holding** (a wrongful commit costs more to unwind than a delayed one).
  - **Surface, never silent:** report open/uncommitted arcs at session end or on any explicit stop/done signal, so nothing is lost. "Human moved on" is acceptance-by-proxy, not real review — this cuts premature commits, it does not guarantee correctness.
  - Explicit overrides win: "commit now" / "don't commit yet" is obeyed immediately, no classification.
- **Pushing to `main` is pre-approved (v0, single dev).** No branch or review gate yet — commit and `git push` directly. Don't ask permission each time. Revisit this rule the moment a second contributor joins.

### Output & Next Directions
- Be direct: state what was found, what's missing, and what change is needed. Don't narrate internal deliberation.
- After a substantive answer or change, append a short numbered list (`1.` / `2.` / `3.`) of plausible next directions so the user can reply by number.
- Offer real alternatives, not one padded path: at least the top next step in the current arc **plus** one genuine pivot. 2–3 entries, each one line naming a concrete surface.
- Never pad to a fixed count; never fill with housekeeping ("make the next commit", "open an issue"). If you can only think of one direction, think harder about the real alternatives.
