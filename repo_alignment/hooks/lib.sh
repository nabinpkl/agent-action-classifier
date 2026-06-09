# shellcheck shell=sh
# Shared helpers for the repo-alignment hooks (POSIX sh; no bashisms). Sourced by
# pre_edit_nudge.sh and end_of_turn_review.sh so the JSON parsing, the cross-provider
# path normalization, and the bounded transcript tail-scan live in exactly one place.
#
# Failure policy: the hooks NEVER deny an agent action. The only hard behavior is
# FAIL LOUD on our OWN failure (jq missing, unparseable input, an unexpected transcript
# shape): surface a systemMessage and exit 1 (visible, non-blocking), never a silent
# exit 0. Compute-helpers signal failure with a non-zero RETURN; only a hook's top level
# calls aa_fail_loud, so the loud message reaches the real hook stdout (never a subshell).

# Cap the tail-scan so cost is bounded regardless of how long an autonomous turn ran.
AA_MAX_SCAN=4000

# Emit a user-visible systemMessage and exit the script. Call ONLY from a hook's top
# level (not inside $()), or the message is captured into a variable instead of emitted.
aa_fail_loud() {
	jq -n --arg m "repo-alignment hook could not run: $1" '{systemMessage: $m}'
	exit 1
}

# Guard: jq is the one hard prerequisite. Without it we cannot build JSON output, so emit
# a plain-JSON systemMessage by hand and fail loud.
aa_require_jq() {
	if ! command -v jq >/dev/null 2>&1; then
		printf '{"systemMessage":"repo-alignment hook: jq not found on PATH (hook disabled)"}\n'
		exit 1
	fi
}

# aa_field INPUT FILTER -> raw value of a jq filter over the hook stdin.
aa_field() {
	printf '%s' "$1" | jq -r "$2"
}

# aa_json_ok INPUT -> return 0 if stdin is valid JSON (the control-char bug,
# claude-code#53463, fails here). The caller fails loud on non-zero.
aa_json_ok() {
	printf '%s' "$1" | jq -e . >/dev/null 2>&1
}

# aa_edited_paths INPUT TOOL -> the file path(s) the tool edits, one per line (empty if
# the tool is not an edit). Claude: Edit/Write/MultiEdit carry .tool_input.file_path.
# Codex: apply_patch carries a patch body whose headers name the files.
aa_edited_paths() {
	case "$2" in
	Edit | Write | MultiEdit)
		printf '%s' "$1" | jq -r '.tool_input.file_path // empty'
		;;
	apply_patch)
		# Codex 0.137 carries the raw patch body in .tool_input.command; older/other
		# builds may use a different field. Collect every string value in tool_input and
		# join with real newlines, so the `*** Update File:` headers land at line starts
		# regardless of the field name, then read the paths. (jq -r '.tool_input' on the
		# OBJECT emits escaped \n with the headers mid-line, which never matched.)
		# sed -E (ERE): BSD/macOS sed has no `\|` BRE alternation (GNU-only), so the old
		# `\(Add\|Update\|Delete\)` silently matched nothing on macOS.
		printf '%s' "$1" | jq -r '[.tool_input | .. | strings] | join("\n")' 2>/dev/null |
			sed -nE 's/^\*\*\* (Add|Update|Delete) File: //p'
		;;
	*) : ;;
	esac
}

# aa_reverse: reverse stdin line order (portable; macOS has `tail -r`, Linux has `tac`,
# awk is everywhere). Input is already bounded to AA_MAX_SCAN lines by the caller.
aa_reverse() {
	awk '{ a[NR] = $0 } END { for (i = NR; i >= 1; i--) print a[i] }'
}

# aa_turn_tuples INPUT -> the bounded, reversed (newest-first) transcript as compact
# tuples on stdout, one per line: "B" for the turn boundary (the last real user prompt),
# or "E\t<path>" for an edit tool_use. RETURNS non-zero (and emits nothing usable) if the
# transcript is missing or jq cannot parse it; the caller fails loud on that.
aa_turn_tuples() {
	# Boundary = a real user prompt: type "user", no toolUseResult, promptSource set
	# (tool results carry toolUseResult and promptSource null; verified against a live
	# transcript 2026-06-07). Edits = assistant tool_use of Edit/Write/MultiEdit.
	aa_tt_filter='
	  if (.type=="user") and (has("toolUseResult")|not) and (.promptSource!=null) then
	    "B"
	  elif (.type=="assistant") then
	    ( .message.content[]?
	      | select(.type=="tool_use")
	      | select(.name=="Edit" or .name=="Write" or .name=="MultiEdit")
	      | "E\t" + ((.input.file_path) // "") )
	  else empty end'

	aa_tt_path=$(printf '%s' "$1" | jq -r '.transcript_path // empty')
	[ -n "$aa_tt_path" ] && [ -f "$aa_tt_path" ] || return 1
	tail -n "$AA_MAX_SCAN" "$aa_tt_path" | aa_reverse | jq -r "$aa_tt_filter" 2>/dev/null
}

# aa_prior_edit_this_turn TUPLES AREA -> return 0 if a prior edit of AREA already happened
# this turn (caller stays quiet), else return 1 (first edit of the area -> nudge). Scans
# newest-first, stopping at the first same-area edit or the turn boundary; cap reached with
# neither -> return 1 (nudge once). Runs in the caller's shell so `return` is real.
aa_prior_edit_this_turn() {
	aa_pe_target=$2
	aa_pe_tab=$(printf '\t')
	aa_pe_oifs=$IFS
	IFS='
'
	# shellcheck disable=SC2086
	set -- $1
	IFS=$aa_pe_oifs
	for aa_pe_line in "$@"; do
		case "$aa_pe_line" in
		B) return 1 ;;
		"E${aa_pe_tab}"*)
			[ "$(path_to_area "${aa_pe_line#E${aa_pe_tab}}")" = "$aa_pe_target" ] && return 0
			;;
		esac
	done
	return 1
}

# aa_turn_touched_areas TUPLES -> distinct skill-areas edited this turn (space-separated)
# on stdout, scanning newest-first back to the boundary. Empty => doc/chat-only turn.
aa_turn_touched_areas() {
	aa_ta_tab=$(printf '\t')
	aa_ta_seen=""
	aa_ta_oifs=$IFS
	IFS='
'
	# shellcheck disable=SC2086
	set -- $1
	IFS=$aa_ta_oifs
	for aa_ta_line in "$@"; do
		case "$aa_ta_line" in
		B) break ;;
		"E${aa_ta_tab}"*)
			aa_ta_area=$(path_to_area "${aa_ta_line#E${aa_ta_tab}}")
			[ -n "$aa_ta_area" ] || continue
			case " $aa_ta_seen " in
			*" $aa_ta_area "*) : ;;
			*) aa_ta_seen="$aa_ta_seen $aa_ta_area" ;;
			esac
			;;
		esac
	done
	printf '%s' "${aa_ta_seen# }"
}
