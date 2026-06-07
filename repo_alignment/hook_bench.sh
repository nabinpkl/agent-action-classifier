#!/bin/sh
# Bench the pre-edit hook's bounded tail-scan: synthetic transcripts of growing size with
# the turn boundary at the OLDEST line (worst case: the scan walks all the way back), then
# time the hook end-to-end. Confirms the AA_MAX_SCAN cap keeps wall-time flat as the
# transcript grows, and surfaces the shell+jq startup floor. perl is used only for hi-res
# timing here (bench-only; NOT a hook dependency).

set -u
DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
HOOK="$DIR/pre_edit_nudge.sh"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
RUNS=50

command -v perl >/dev/null 2>&1 || {
	echo "hook_bench: perl needed for hi-res timing (bench only)" >&2
	exit 1
}

# gen N FILE: a transcript of N lines, boundary (real user prompt) at line 1, then N-1
# filler assistant Read tool_uses (no rust edit), so the scan reaches the boundary.
gen() {
	{
		printf '{"type":"user","promptSource":"typed","message":{"role":"user","content":"go"}}\n'
		i=1
		while [ "$i" -lt "$1" ]; do
			printf '{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Read","input":{"file_path":"x.txt"}}]}}\n'
			i=$((i + 1))
		done
	} >"$2"
}

now() { perl -MTime::HiRes=time -e 'printf "%.6f", time'; }

echo "pre-edit hook scan cost vs transcript size (boundary at oldest line, $RUNS runs each):"
for n in 100 1000 10000 50000; do
	tf="$TMP/t_$n.jsonl"
	gen "$n" "$tf"
	input=$(printf '{"tool_name":"Edit","tool_input":{"file_path":"bench.rs"},"transcript_path":"%s"}' "$tf")
	s=$(now)
	i=0
	while [ "$i" -lt "$RUNS" ]; do
		printf '%s' "$input" | "$HOOK" >/dev/null 2>&1
		i=$((i + 1))
	done
	e=$(now)
	awk -v s="$s" -v e="$e" -v r="$RUNS" -v n="$n" \
		'BEGIN { printf "  N=%-6d  %.2f ms/call\n", n, (e - s) / r * 1000 }'
done
