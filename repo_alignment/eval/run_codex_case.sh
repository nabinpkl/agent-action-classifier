#!/bin/sh
# Run ONE eval case through the subject agent (Codex) in an isolated git worktree
# under a given nudge condition, then grade it (ADR-0014).
#
# Usage: run_codex_case.sh <case-id> <off|on> <base-ref> <out-dir>
#
# The ONLY difference between the two conditions is whether the pre-edit nudge
# fires: ON keeps .codex/hooks.json in the worktree, OFF moves it aside. Codex is
# invoked identically otherwise, so the paired delta isolates the nudge's effect.
#
# Emits <out-dir>/verdict.json. Deterministic-lane cases are graded here;
# judge-lane cases bundle the diff + transcript for the pinned Claude judge and
# leave violated:null. Fail loud on our own failure (exit 2), never a silent pass.

set -eu

die() { echo "run_codex_case: $*" >&2; exit 2; }
command -v jq >/dev/null 2>&1 || die "jq not found"
command -v codex >/dev/null 2>&1 || die "codex not found"

CASE_ID=${1:?usage: run_codex_case.sh <case-id> <off|on> <base-ref> <out-dir>}
COND=${2:?condition off|on required}
BASE=${3:?base ref required}
OUT=${4:?out-dir required}
case "$COND" in off|on) ;; *) die "condition must be off|on, got $COND" ;; esac

ROOT=$(git rev-parse --show-toplevel) || die "not a git repo"
CASES="$ROOT/repo_alignment/eval/cases/cases.json"
GRADER="$ROOT/repo_alignment/eval/case_grader.sh"
CASE=$(jq -c --arg id "$CASE_ID" '.cases[] | select(.id==$id)' "$CASES")
[ -n "$CASE" ] || die "unknown case id: $CASE_ID"
PROMPT=$(printf '%s' "$CASE" | jq -r '.prompt')
LANE=$(printf '%s' "$CASE" | jq -r '.lane')

mkdir -p "$OUT"
WT=$(mktemp -d)/wt
git worktree add -q --detach "$WT" "$BASE" || die "worktree add failed"
# shellcheck disable=SC2064
trap "git worktree remove --force '$WT' 2>/dev/null || true" EXIT

# Toggle the single experimental variable.
if [ "$COND" = "off" ] && [ -d "$WT/.codex" ]; then
	mv "$WT/.codex" "$WT/.codex.disabled"
fi

# Subject run: Codex, headless, scoped to the worktree, allowed to edit it.
codex exec \
	-C "$WT" \
	--sandbox workspace-write \
	--dangerously-bypass-hook-trust \
	--skip-git-repo-check \
	"$PROMPT" >"$OUT/transcript.txt" 2>&1 || die "codex exec failed (see $OUT/transcript.txt)"

# Make any new files visible to the diff-based grader.
git -C "$WT" add -A 2>/dev/null || true
git -C "$WT" diff --cached "$BASE" >"$OUT/diff.patch" 2>/dev/null || true

if [ "$LANE" = "judge" ]; then
	# Bundle for the pinned Claude judge; verdict stays null until judged.
	jq -n --arg id "$CASE_ID" --arg cond "$COND" \
		'{id:$id, condition:$cond, lane:"judge", violated:null,
		  bundle:{diff:"diff.patch", transcript:"transcript.txt"}}' >"$OUT/verdict.json"
else
	"$GRADER" "$CASE_ID" "$BASE" "$WT" |
		jq --arg cond "$COND" '. + {condition:$cond}' >"$OUT/verdict.json"
fi

cat "$OUT/verdict.json"
