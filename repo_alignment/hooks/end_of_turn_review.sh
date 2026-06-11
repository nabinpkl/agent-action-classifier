#!/bin/sh
# Stop hook: on a code-touching turn, elicit a concise, risk-scaled self-review plus next
# directions in the agent's final message. Detail scales to the change: mundane work stays
# brief, risks/tradeoffs get called out. It blocks only the agent's *stop* once
# (stop_hook_active guards a second pass), never an edit or a user action. Fails loud only
# on our own failure. The human reading the acknowledgment is the gate. See docs/adr/0013.

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

# Build the relevant-skill pointers for the review instruction from skills_for_area (the
# single source of truth in skill_map.sh), deduped across the touched areas. Attention
# pointers only: name the skills, do not prescribe techniques. /pydantic-models is not
# listed here; it is a conditional pointer in the pre-edit reminder for when data models
# are actually involved.
SKILLS=""
for a in $AREAS; do
	for s in $(skills_for_area "$a"); do
		case " $SKILLS " in
		*" $s "*) : ;;
		*) SKILLS="$SKILLS $s" ;;
		esac
	done
done
SKILLS=${SKILLS# }

REASON="This turn changed: $AREAS. In your FINAL message, keep it proportional to the change: (1) briefly say what changed and which of the relevant skills ($SKILLS) shaped it — one or two lines is enough for mundane work, do not force a justification; (2) call out any real risk, tradeoff, or deliberate non-obvious choice plainly, and skip this if there is none; (3) if a rule or nudge looks stale versus the architecture, flag it; (4) end with the AGENTS.md next-directions list — 2-3 one-line numbered options, the top next step in this arc plus at least one genuine pivot."

# Block-to-continue via exit 2 + reason on stderr. This is the cross-runtime mechanism:
# Codex 0.137 rejects the JSON {decision:"block"} shape ("invalid stop hook JSON output",
# docs-ahead-of-binary), but both Codex and Claude honor exit 2 + stderr as the
# continuation signal. stop_hook_active (above) still guards exactly one extra pass.
printf '%s\n' "$REASON" >&2
exit 2
