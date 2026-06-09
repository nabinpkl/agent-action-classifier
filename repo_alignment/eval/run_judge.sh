#!/bin/sh
# Score one judge-lane eval case with the pinned Claude judge (ADR-0014). Reads the
# bundle run_codex_case.sh produced (diff.patch + transcript.jsonl) plus the case's
# TARGET_RULE + RUBRIC, runs the pinned judge with judge_prompt.md, and writes
# violated:0|1 back into verdict.json. Subject = Codex, judge = Claude -> no self-eval.
#
# Usage: run_judge.sh <out-dir>
# Fail loud on our own failure (exit 2), never a silent pass.

set -eu

die() {
	echo "run_judge: $*" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || die "jq not found"
command -v claude >/dev/null 2>&1 || die "claude not found"

OUT=${1:?usage: run_judge.sh <out-dir>}
[ -f "$OUT/verdict.json" ] || die "no verdict.json in $OUT"

ROOT=$(git rev-parse --show-toplevel) || die "not a git repo"
CASES="$ROOT/repo_alignment/eval/cases/cases.json"
PROMPT_MD="$ROOT/repo_alignment/eval/judge_prompt.md"
JUDGE_MODEL=${JUDGE_MODEL:-claude-opus-4-8}

id=$(jq -r '.id' "$OUT/verdict.json")
lane=$(jq -r '.lane' "$OUT/verdict.json")
[ "$lane" = judge ] || die "case $id is not a judge-lane case"

CASE=$(jq -c --arg id "$id" '.cases[] | select(.id==$id)' "$CASES")
[ -n "$CASE" ] || die "unknown case id: $id"
TARGET=$(printf '%s' "$CASE" | jq -r '.target_rule')
RUBRIC=$(printf '%s' "$CASE" | jq -r '.rubric')
DIFF=$(cat "$OUT/diff.patch" 2>/dev/null || true)
TRANSCRIPT=$(cat "$OUT/transcript.jsonl" 2>/dev/null || true)
[ -n "$DIFF$TRANSCRIPT" ] || die "empty bundle (no diff or transcript) for $id"

PROMPT=$(printf '%s\n\n## CASE\nid: %s\nTARGET_RULE: %s\nRUBRIC: %s\n\n## diff.patch\n%s\n\n## transcript.jsonl\n%s\n' \
	"$(cat "$PROMPT_MD")" "$id" "$TARGET" "$RUBRIC" "$DIFF" "$TRANSCRIPT")

RAW=$(printf '%s' "$PROMPT" | claude -p --model "$JUDGE_MODEL" --output-format text 2>/dev/null) ||
	die "claude judge invocation failed for $id"

# The judge is told to emit ONLY the strict JSON. Strip any code fences, then take the
# one object carrying a "violated" bit; fall back to an inline {...} if it added prose.
CLEAN=$(printf '%s' "$RAW" | sed '/^```/d')
VERDICT=$(printf '%s' "$CLEAN" | jq -c 'select(.violated != null)' 2>/dev/null | head -1)
[ -n "$VERDICT" ] ||
	VERDICT=$(printf '%s' "$RAW" | sed -n 's/.*\({[^{}]*"violated"[^{}]*}\).*/\1/p' | head -1)
[ -n "$VERDICT" ] || die "judge output had no parseable verdict for $id: $(printf '%s' "$RAW" | head -3)"

V=$(printf '%s' "$VERDICT" | jq -r '.violated')
case "$V" in 0 | 1) ;; *) die "judge returned non-bit violated for $id: $V" ;; esac

jq --argjson v "$V" --argjson j "$VERDICT" '. + {violated:$v, judge:$j}' "$OUT/verdict.json" >"$OUT/verdict.json.tmp"
mv "$OUT/verdict.json.tmp" "$OUT/verdict.json"
cat "$OUT/verdict.json"
