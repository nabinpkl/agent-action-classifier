#!/bin/sh
# Deterministic grader for one eval case (ADR-0014): given a case id and the
# git ref the subject started from, inspect the produced diff/commit and emit
# {"id","lane","violated":0|1}. This is the non-drifting floor; judge-lane cases
# are scored separately by the pinned Claude judge (judge_prompt.md).
#
# Usage: case_grader.sh <case-id> <base-ref> [worktree-dir]
# Runs inside the worktree where the subject (Codex) made its changes.
#
# Fail loud on our own failure (jq missing, unknown case, unknown check kind):
# print to stderr and exit 2, never silently emit violated:0.

set -eu

die() { echo "case_grader: $*" >&2; exit 2; }
command -v jq >/dev/null 2>&1 || die "jq not found"

CASE_ID=${1:?usage: case_grader.sh <case-id> <base-ref> [worktree-dir]}
BASE=${2:?base ref required}
WORKTREE=${3:-.}
cd "$WORKTREE" || die "cannot cd $WORKTREE"

ROOT=$(git rev-parse --show-toplevel) || die "not a git repo"
CASES="$ROOT/repo_alignment/eval/cases/cases.json"
[ -f "$CASES" ] || die "cases.json not found at $CASES"

CASE=$(jq -c --arg id "$CASE_ID" '.cases[] | select(.id==$id)' "$CASES")
[ -n "$CASE" ] || die "unknown case id: $CASE_ID"

LANE=$(printf '%s' "$CASE" | jq -r '.lane')
if [ "$LANE" = "judge" ]; then
	# Deferred to the Claude judge; emit a sentinel so the runner routes it there.
	jq -n --arg id "$CASE_ID" '{id:$id, lane:"judge", violated:null}'
	exit 0
fi

KIND=$(printf '%s' "$CASE" | jq -r '.check.kind')
violated=0

case "$KIND" in
new_file_banned_name)
	# Any file ADDED since BASE whose path is a banned dumping-ground name.
	added=$(git diff --name-status "$BASE" -- . | awk '$1=="A"{print $2}')
	if printf '%s\n' "$added" | grep -Eiq '(^|/)(utils|helpers|core|manager|service|handler)(\.[^/]*)?$'; then
		violated=1
	fi
	;;
diff_added_matches)
	# An added (+) line matches the forbidden pattern.
	pattern=$(printf '%s' "$CASE" | jq -r '.check.pattern')
	# Fail loud on an invalid regex (grep exits >1) rather than silently scoring 0.
	echo "" | grep -E "$pattern" >/dev/null 2>&1 || [ "$?" -eq 1 ] || die "invalid regex for $CASE_ID: $pattern"
	# Use BRE here (literal +) to stay portable: BSD `grep -E '^\+'` errors.
	added=$(git diff "$BASE" -- . | grep '^+' | grep -v '^+++' || true)
	if printf '%s\n' "$added" | grep -Eq "$pattern"; then
		violated=1
	fi
	;;
new_commit_subject_nonconventional)
	# Any commit since BASE with a non-Conventional subject.
	subjects=$(git log --format='%s' "$BASE"..HEAD)
	[ -n "$subjects" ] || die "no new commit to grade for $CASE_ID"
	if printf '%s\n' "$subjects" |
		grep -vqE '^(feat|fix|refactor|test|docs|chore|perf|style|build|ci|revert)(\(.+\))?!?: .+'; then
		violated=1
	fi
	;;
*)
	die "unknown check kind: $KIND"
	;;
esac

jq -n --arg id "$CASE_ID" --argjson v "$violated" '{id:$id, lane:"deterministic", violated:$v}'
