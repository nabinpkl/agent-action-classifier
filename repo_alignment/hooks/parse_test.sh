#!/bin/sh
# Unit test for aa_edited_paths (lib.sh): the payload-shape coupling that silently
# no-opped the Codex nudge once before. Feeds the real captured Codex 0.137 apply_patch
# payload and the Claude Edit shape, asserts the extracted path. Fail loud on mismatch.
#
# Run: repo_alignment/hooks/parse_test.sh   (or `just parse-test`)

set -eu

case "$0" in */*) d=${0%/*} ;; *) d=. ;; esac
DIR=$(CDPATH= cd -- "$d" && pwd)
. "$DIR/skill_map.sh"
. "$DIR/lib.sh"

fail() {
	echo "parse_test FAIL: $1" >&2
	echo "  expected: $2" >&2
	echo "  actual:   $3" >&2
	exit 1
}

# 1) Codex 0.137 apply_patch: patch body in .tool_input.command, headers inside the string.
codex_input='{"tool_name":"apply_patch","tool_input":{"command":"*** Begin Patch\n*** Update File: crates/policy_decision/src/policy.rs\n@@\n #[derive(Debug)]\n+/// rule identifier\n pub struct RuleId(pub String);\n*** End Patch\n"}}'
got=$(aa_edited_paths "$codex_input" apply_patch)
[ "$got" = "crates/policy_decision/src/policy.rs" ] ||
	fail "codex apply_patch path" "crates/policy_decision/src/policy.rs" "$got"

# 2) Codex apply_patch with multiple file headers (Add + Update) -> both paths, in order.
codex_multi='{"tool_name":"apply_patch","tool_input":{"command":"*** Begin Patch\n*** Add File: a/new_thing.rs\n+fn x() {}\n*** Update File: b/existing.rs\n+// note\n*** End Patch\n"}}'
got=$(aa_edited_paths "$codex_multi" apply_patch | tr '\n' ',')
[ "$got" = "a/new_thing.rs,b/existing.rs," ] ||
	fail "codex apply_patch multi path" "a/new_thing.rs,b/existing.rs," "$got"

# 3) Claude Edit shape: path in .tool_input.file_path (must still work).
claude_input='{"tool_name":"Edit","tool_input":{"file_path":"src/host/decide.py","old_string":"a","new_string":"b"}}'
got=$(aa_edited_paths "$claude_input" Edit)
[ "$got" = "src/host/decide.py" ] ||
	fail "claude Edit path" "src/host/decide.py" "$got"

# 4) Non-edit tool -> empty (no nudge).
got=$(aa_edited_paths '{"tool_name":"Bash","tool_input":{"command":"ls"}}' Bash)
[ -z "$got" ] || fail "non-edit tool empty" "(empty)" "$got"

# --- aa_turn_tuples: the transcript-file scan, both runtimes (the Stop-hook path) ---
# Records are written oldest-first (as a real transcript is); aa_turn_tuples tail+reverses.

# 5) Codex rollout: user_message boundary + an apply_patch custom_tool_call touching a
# .rs then a .py. Expect tuples newest-first: E rs, E py, B; touched areas "rust python".
codex_tx=$(mktemp)
jq -nc '{type:"event_msg", payload:{type:"user_message", message:"add things"}}' >"$codex_tx"
jq -nc '{type:"response_item", payload:{type:"custom_tool_call", name:"apply_patch",
	input:"*** Begin Patch\n*** Update File: crates/policy_decision/src/policy.rs\n+// a\n*** Add File: src/host/decide.py\n+x=1\n*** End Patch\n"}}' >>"$codex_tx"
codex_payload=$(jq -nc --arg t "$codex_tx" '{transcript_path:$t}')
tuples=$(aa_turn_tuples "$codex_payload") || fail "codex aa_turn_tuples returned nonzero" "tuples" "(rc!=0)"
got=$(printf '%s' "$tuples" | tr '\n' '|')
case "$got" in
*"E	crates/policy_decision/src/policy.rs"*"E	src/host/decide.py"*"B"*) : ;;
*) fail "codex tuples" "E rs | E py | B" "$got" ;;
esac
areas=$(aa_turn_touched_areas "$tuples")
[ "$areas" = "rust python" ] || fail "codex touched areas" "rust python" "$areas"
rm -f "$codex_tx"

# 6) Claude transcript: a real user prompt boundary + an assistant Edit of a .rs.
claude_tx=$(mktemp)
jq -nc '{type:"user", promptSource:"user", message:{content:[{type:"text",text:"go"}]}}' >"$claude_tx"
jq -nc '{type:"assistant", message:{content:[{type:"tool_use", name:"Edit",
	input:{file_path:"crates/policy_decision/src/decision.rs"}}]}}' >>"$claude_tx"
claude_payload=$(jq -nc --arg t "$claude_tx" '{transcript_path:$t}')
tuples=$(aa_turn_tuples "$claude_payload") || fail "claude aa_turn_tuples returned nonzero" "tuples" "(rc!=0)"
areas=$(aa_turn_touched_areas "$tuples")
[ "$areas" = "rust" ] || fail "claude touched areas" "rust" "$areas"
rm -f "$claude_tx"

echo "parse_test OK (6 assertions)"
