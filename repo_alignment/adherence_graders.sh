#!/bin/sh
# Deterministic adherence graders: the countable AGENTS.md rules. A post-hoc AUDIT and the
# human's evidence when judging a turn, NOT an edit-time gate (the hooks never deny edits).
# Exit 1 on any violation. Reuses git/grep; file-size is already the .githooks/pre-commit
# check and dead-code is `just check` clippy, so neither is duplicated here.
#
# v1 floor is deliberately small: only rules that are UNAMBIGUOUS and machine-checkable
# live here. Em-dashes are NOT graded (that is a chat-response rule, not a repo rule;
# AGENTS.md itself uses them). The bulk of AGENTS.md is judgment-level and belongs to the
# (deferred) LLM judge; until then the evidence-backed final-message acknowledgment surfaces
# it for the human.

root=$(git rev-parse --show-toplevel) || exit 1
cd "$root" || exit 1
fail=0

echo "── adherence: banned dumping-ground names ──"
banned=$(git ls-files | grep -Ei '(^|/)(utils|helpers|core|manager|service|handler)(\.[^/]*)?(/|$)' || true)
if [ -n "$banned" ]; then
	echo "  ✗ banned concept name(s) (AGENTS.md) — use a literal concept name:"
	printf '%s\n' "$banned" | sed 's/^/      /'
	fail=1
else
	echo "  ✓ none"
fi

echo "── adherence: conventional-commit subjects ──"
bad=$(git log --format='%s' |
	grep -vE '^(feat|fix|refactor|test|docs|chore|perf|style|build|ci|revert)(\(.+\))?!?: .+' || true)
if [ -n "$bad" ]; then
	echo "  ✗ non-conventional commit subject(s):"
	printf '%s\n' "$bad" | sed 's/^/      /'
	fail=1
else
	echo "  ✓ all conventional"
fi

echo ""
if [ "$fail" -eq 0 ]; then
	echo "adherence: OK"
else
	echo "adherence: VIOLATIONS (see above)"
fi
exit "$fail"
