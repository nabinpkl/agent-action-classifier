#!/bin/sh
# Stop hook: on a code-touching turn, elicit a one-pass, evidence-backed self-review in
# the agent's final message. It blocks only the agent's *stop* once (stop_hook_active
# guards a second pass), never an edit or a user action. Fails loud only on our own
# failure. The human reading the acknowledgment is the gate. See docs/adr/0013.

case "$0" in */*) d=${0%/*} ;; *) d=. ;; esac
DIR=$(CDPATH= cd -- "$d" && pwd)
. "$DIR/skill_map.sh"
. "$DIR/lib.sh"

aa_require_jq
INPUT=$(cat)
aa_json_ok "$INPUT" || aa_fail_loud "stdin was not valid JSON"

# Loop guard: if our review continuation already ran this turn, let the turn end.
ACTIVE=$(aa_field "$INPUT" '.stop_hook_active // false')
[ "$ACTIVE" = "true" ] && exit 0

TUPLES=$(aa_turn_tuples "$INPUT") || aa_fail_loud "could not read/parse the transcript"
AREAS=$(aa_turn_touched_areas "$TUPLES")
[ -n "$AREAS" ] || exit 0 # doc/chat-only turn: stay quiet

# Build the per-area skill pointers for the review instruction. Attention pointers only:
# name the relevant skills, do not prescribe specific techniques (the skill is the source
# of truth). /pydantic-models is intentionally not listed by default; it is a conditional
# pointer in the pre-edit reminder for when data models are actually involved.
SKILLS=""
for a in $AREAS; do
	case "$a" in
	rust) SKILLS="$SKILLS /rust-coding" ;;
	python) SKILLS="$SKILLS /python-dev-tooling" ;;
	esac
done
SKILLS="$SKILLS /source-code-organization"

REASON="This turn changed: $AREAS. In your FINAL message, give an evidence-backed self-review, not a bare \"done\": (1) which of the relevant skills ($SKILLS) you consulted and HOW each concretely shaped the change; (2) what you CHANGED to comply with AGENTS.md and those skills, and what you deliberately did NOT change and why (justification); (3) if a rule or nudge looks stale versus the architecture, surface it to the user."

# Block-to-continue via exit 2 + reason on stderr. This is the cross-runtime mechanism:
# Codex 0.137 rejects the JSON {decision:"block"} shape ("invalid stop hook JSON output",
# docs-ahead-of-binary), but both Codex and Claude honor exit 2 + stderr as the
# continuation signal. stop_hook_active (above) still guards exactly one extra pass.
printf '%s\n' "$REASON" >&2
exit 2
