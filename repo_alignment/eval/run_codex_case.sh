#!/bin/sh
# Run ONE eval case through the subject agent (interactive Codex via tmux) in an
# isolated git worktree under a given nudge condition, then grade it (ADR-0014/0015).
#
# Usage: run_codex_case.sh <case-id> <off|on> <base-ref> <out-dir>
#
# The ONLY difference between off and on is whether the pre-edit nudge fires: ON keeps
# .codex/hooks.json in the worktree (the hook injects the reminder), OFF moves it aside.
# Codex is invoked identically otherwise, so the paired delta isolates the nudge.
#
# Subject = interactive Codex (NOT `codex exec`): Codex 0.137 fires hooks only
# interactively (ADR-0015), so exec cannot deliver the nudge. We drive a tmux session
# via codex_session.sh, capture the diff + transcript, and grade before teardown.
#
# Emits <out-dir>/verdict.json (+ diff.patch, untracked.txt, transcript path, meta).
# Fail loud on our own failure (exit 2), never a silent pass.

set -eu

die() {
	echo "run_codex_case: $*" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || die "jq not found"
command -v codex >/dev/null 2>&1 || die "codex not found"
command -v tmux >/dev/null 2>&1 || die "tmux not found"

CASE_ID=${1:?usage: run_codex_case.sh <case-id> <off|on> <base-ref> <out-dir>}
COND=${2:?condition off|on required}
BASE=${3:?base ref required}
OUT=${4:?out-dir required}
case "$COND" in off | on) ;; *) die "condition must be off|on, got $COND" ;; esac

ROOT=$(git rev-parse --show-toplevel) || die "not a git repo"
. "$ROOT/repo_alignment/eval/codex_session.sh"
CASES="$ROOT/repo_alignment/eval/cases/cases.json"
GRADER="$ROOT/repo_alignment/eval/case_grader.sh"
CASE=$(jq -c --arg id "$CASE_ID" '.cases[] | select(.id==$id)' "$CASES")
[ -n "$CASE" ] || die "unknown case id: $CASE_ID"
PROMPT=$(printf '%s' "$CASE" | jq -r '.prompt')
LANE=$(printf '%s' "$CASE" | jq -r '.lane')
KIND=$(printf '%s' "$CASE" | jq -r '.check.kind // empty')

BASE_SHA=$(git rev-parse "$BASE") || die "cannot resolve base ref: $BASE"
mkdir -p "$OUT"
WT=$(mktemp -d)/wt
SESSION="eval-$CASE_ID-$COND"
git worktree add -q --detach "$WT" "$BASE_SHA" || die "worktree add failed"
# shellcheck disable=SC2064
trap "cs_teardown '$SESSION'; git worktree remove --force '$WT' 2>/dev/null || true; git worktree prune 2>/dev/null || true" EXIT

# Toggle the single experimental variable.
if [ "$COND" = "off" ] && [ -d "$WT/.codex" ]; then
	mv "$WT/.codex" "$WT/.codex.disabled"
fi

# Build the brief. Most cases are edit-only and must NOT run build/test/git; the lone
# commit case needs to commit, so it gets a permissive launch (-a never) instead of an
# approval dance, and its fence allows the commit.
BRIEF=$(mktemp)
if [ "$KIND" = "new_commit_subject_nonconventional" ]; then
	export CS_LAUNCH='codex -s workspace-write -a never'
	{
		printf '%s\n\n' "$PROMPT"
		printf 'Make the change with apply_patch, then commit it. Do not push.\n'
	} >"$BRIEF"
else
	{
		printf '%s\n\n' "$PROMPT"
		printf 'Make the edit with apply_patch. Do NOT run cargo, tests, or git. Do NOT commit. Just make the edit and stop.\n'
	} >"$BRIEF"
fi

# Marker to find the transcript this run produces (newest rollout created after launch).
MARKER=$(mktemp)

cs_bring_up "$SESSION" "$WT" || die "bring-up failed for $SESSION"
cs_dispatch "$SESSION" "$BRIEF" || die "dispatch failed for $SESSION"
STATUS=$(cs_monitor "$SESSION")

# Capture artifacts (robust to Codex committing: diff against the base SHA). Exclude
# .codex/.codex.disabled: the nudge toggle (the OFF arm moves .codex aside) is eval
# infrastructure, not the subject's change, and would otherwise pollute the diff/judge.
git -C "$WT" diff "$BASE_SHA" -- . ':!.codex' ':!.codex.disabled' >"$OUT/diff.patch" 2>/dev/null || true
# Untracked additions (a plain diff hides them; the banned-name grader needs them).
( cd "$WT" && git ls-files --others --exclude-standard -- . ':!.codex.disabled' ) >"$OUT/untracked.txt" 2>/dev/null || true
# Transcript: newest Codex rollout created after the marker.
TRANSCRIPT=$(find "$HOME/.codex/sessions" -name 'rollout-*.jsonl' -newer "$MARKER" 2>/dev/null \
	| while IFS= read -r f; do printf '%s\t%s\n' "$(stat -f '%m' "$f" 2>/dev/null || echo 0)" "$f"; done \
	| sort -rn | head -1 | cut -f2-)
[ -n "$TRANSCRIPT" ] && cp "$TRANSCRIPT" "$OUT/transcript.jsonl" 2>/dev/null || true

DEVIATED=false
[ "$STATUS" = DEVIATED ] && DEVIATED=true
case "$STATUS" in
DONE | DEVIATED | TIMEOUT) ;;
*) die "unexpected monitor status: $STATUS" ;;
esac

# Grade while the worktree is still alive (case_grader runs git inside it).
if [ "$LANE" = "judge" ]; then
	jq -n --arg id "$CASE_ID" --arg cond "$COND" --arg st "$STATUS" --argjson dev "$DEVIATED" \
		'{id:$id, condition:$cond, lane:"judge", violated:null, status:$st, deviated:$dev,
		  bundle:{diff:"diff.patch", transcript:"transcript.jsonl"}}' >"$OUT/verdict.json"
else
	"$GRADER" "$CASE_ID" "$BASE_SHA" "$WT" \
		| jq --arg cond "$COND" --arg st "$STATUS" --argjson dev "$DEVIATED" \
			'. + {condition:$cond, status:$st, deviated:$dev}' >"$OUT/verdict.json"
fi

rm -f "$BRIEF" "$MARKER"
cat "$OUT/verdict.json"
