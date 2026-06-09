#!/bin/sh
# Batch-run one experiment over the case bank (ADR-0014/0015). For each case, run the
# subject OFF then ON back-to-back (paired, so shared time/ratelimit drift cancels in
# the delta), grade each (deterministic in-runner, judge-lane via run_judge.sh), then
# emit the {cases:[{id,off,on}]} results paired_ci.py consumes and print the verdict.
#
# Usage: run_eval.sh [experiment]    (default E1)
#   OUT_ROOT=<dir>   override artifact root (default /tmp/aa_eval/<exp>)
#   ONLY=<id,id>     restrict to these case ids (calibration / debugging)
#
# Sequential by design: one interactive Codex session at a time. Fail loud on setup
# failure; a single failed/deviated case is logged and excluded, not fatal.

# set -u only (NOT -e): a batch driver must survive one case failing without aborting
# the other 20+. Setup errors are handled explicitly with `|| die`.
set -u

die() {
	echo "run_eval: $*" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || die "jq not found"

# Normalize a verdict file's violated bit to valid JSON (0|1|null) for --argjson; any
# missing/partial/non-bit value -> null (excluded from the CI), never a parse error.
read_bit() {
	v=$(jq -r '.violated' "$1" 2>/dev/null)
	case "$v" in 0 | 1) echo "$v" ;; *) echo null ;; esac
}
read_deviated() {
	v=$(jq -r '.deviated' "$1" 2>/dev/null)
	case "$v" in true) echo true ;; *) echo false ;; esac
}

EXP=${1:-E1}
ROOT=$(git rev-parse --show-toplevel) || die "not a git repo"
RUNNER="$ROOT/repo_alignment/eval/run_codex_case.sh"
JUDGE="$ROOT/repo_alignment/eval/run_judge.sh"
CASES="$ROOT/repo_alignment/eval/cases/cases.json"
ANALYZER="$ROOT/repo_alignment/eval/paired_ci.py"
OUTROOT=${OUT_ROOT:-/tmp/aa_eval/$EXP}

# Reaper: clear stale eval sessions/worktrees from any crashed prior run.
tmux ls 2>/dev/null | awk -F: '/^eval-/{print $1}' | while read -r s; do
	tmux kill-session -t "$s" 2>/dev/null || true
done
git worktree prune 2>/dev/null || true

BASE=$(git rev-parse HEAD)
mkdir -p "$OUTROOT"
echo "experiment=$EXP base=$BASE out=$OUTROOT" >&2

if [ -n "${ONLY:-}" ]; then
	IDS=$(printf '%s' "$ONLY" | tr ',' ' ')
else
	IDS=$(jq -r '.cases[].id' "$CASES")
fi

PAIRS="$OUTROOT/pairs.jsonl"
: >"$PAIRS"
for id in $IDS; do
	for cond in off on; do
		out="$OUTROOT/$id/$cond"
		mkdir -p "$out" || die "cannot create $out"
		echo "[$EXP] $id / $cond ..." >&2
		if "$RUNNER" "$id" "$cond" "$BASE" "$out" >"$out.runner.log" 2>&1; then
			lane=$(jq -r '.lane' "$out/verdict.json" 2>/dev/null || echo '?')
			if [ "$lane" = judge ]; then
				"$JUDGE" "$out" >"$out.judge.log" 2>&1 ||
					echo "  judge failed for $id/$cond (see $out.judge.log)" >&2
			fi
		else
			echo "  runner failed for $id/$cond (see $out.runner.log)" >&2
		fi
	done
	voff=$(read_bit "$OUTROOT/$id/off/verdict.json")
	von=$(read_bit "$OUTROOT/$id/on/verdict.json")
	devoff=$(read_deviated "$OUTROOT/$id/off/verdict.json")
	devon=$(read_deviated "$OUTROOT/$id/on/verdict.json")
	jq -nc --arg id "$id" --argjson off "$voff" --argjson on "$von" \
		--argjson devoff "$devoff" --argjson devon "$devon" \
		'{id:$id, off:$off, on:$on, deviated:($devoff or $devon)}' >>"$PAIRS"
done

# Keep only fully-scored, non-deviated pairs for the CI (null = failed/deviated/unscored).
RESULTS="$OUTROOT/results.json"
jq -s '{cases: [ .[] | select(.off != null and .on != null and (.deviated | not))
		| {id, off, on} ]}' "$PAIRS" >"$RESULTS"

kept=$(jq '.cases | length' "$RESULTS")
total=$(wc -l <"$PAIRS" | tr -d ' ')
echo >&2
echo "scored $kept/$total cases (excluded: failed/deviated/unscored)" >&2
[ "$kept" -gt 0 ] || die "no fully-scored pairs to analyze (see $OUTROOT/*/*.log)"

echo "results: $RESULTS"
python3 "$ANALYZER" "$RESULTS"
