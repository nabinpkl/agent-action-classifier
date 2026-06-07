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

# Build the per-area skill pointers for the review instruction.
SKILLS=""
for a in $AREAS; do
	case "$a" in
	rust) SKILLS="$SKILLS /rust-coding" ;;
	python) SKILLS="$SKILLS /python-dev-tooling /pydantic-models" ;;
	esac
done
SKILLS="$SKILLS /source-code-organization"

REASON="This turn changed: $AREAS. In your FINAL message, give an evidence-backed self-review, not a bare \"done\": (1) which skill nudges fired ($SKILLS) and HOW each concretely shaped the code, citing specific decisions (e.g. thiserror at the lib boundary, exhaustive match, borrowed not cloned); (2) what you CHANGED to comply with AGENTS.md and those skills, and what you deliberately did NOT change and why (justification); (3) if a rule looks stale versus the architecture, surface it to the user."

jq -n --arg r "$REASON" \
	'{decision: "block", reason: $r, hookSpecificOutput: {hookEventName: "Stop"}}'
exit 0
