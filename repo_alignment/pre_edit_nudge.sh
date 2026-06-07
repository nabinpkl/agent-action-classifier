#!/bin/sh
# PreToolUse hook: a soft skill-area nudge on the FIRST edit of an area per turn.
# It NEVER denies the edit (exit 0). It fails loud (systemMessage + exit 1) only when our
# own machinery cannot run (jq missing, unparseable input, unreadable transcript). See
# docs/adr/0013. Wired for Claude (Edit|Write|MultiEdit) and Codex (apply_patch).

case "$0" in */*) d=${0%/*} ;; *) d=. ;; esac
DIR=$(CDPATH= cd -- "$d" && pwd)
. "$DIR/skill_map.sh"
. "$DIR/lib.sh"

aa_require_jq
INPUT=$(cat)
aa_json_ok "$INPUT" || aa_fail_loud "stdin was not valid JSON"

TOOL=$(aa_field "$INPUT" '.tool_name // empty')
PATHS=$(aa_edited_paths "$INPUT" "$TOOL")
[ -n "$PATHS" ] || exit 0 # not an edit tool

# Area for the first edited path that maps to one (apply_patch may list several).
AREA=$(printf '%s\n' "$PATHS" | while IFS= read -r p; do
	a=$(path_to_area "$p")
	[ -n "$a" ] && { printf '%s' "$a"; break; }
done)
[ -n "$AREA" ] || exit 0 # no skill-area for this file type

# Debounce: stay quiet if a prior edit of this area already happened this turn.
TUPLES=$(aa_turn_tuples "$INPUT") || aa_fail_loud "could not read/parse the transcript"
if aa_prior_edit_this_turn "$TUPLES" "$AREA"; then
	exit 0
fi

REMINDER=$(reminder_for_area "$AREA")
[ -n "$REMINDER" ] || exit 0
jq -n --arg ctx "$REMINDER" \
	'{hookSpecificOutput: {hookEventName: "PreToolUse", additionalContext: $ctx}}'
exit 0
