#!/bin/sh
# Drive an interactive Codex session in tmux, unattended, for the eval (ADR-0015).
# Sourced by run_codex_case.sh. This is the scripted analog of the codex-tmux skill:
# the skill is a human driving Codex by hand; this is the eval runner driving it
# headless. Different consumers, so the logic lives here, not shared with the skill.
#
# Why interactive (not `codex exec`): Codex 0.137 fires hooks ONLY in interactive
# mode (proven: exec/app-server -> hook/started=0; interactive -> hook runs). The
# nudge under test cannot reach Codex any other way. See docs/adr/0015.
#
# Functions: cs_bring_up, cs_dispatch, cs_monitor, cs_teardown. Fail loud (exit/return
# non-zero + stderr) on our own failure; never auto-answer an unrecognized prompt.

# Tunables (env-overridable). Eval turns are small single edits, so poll faster than
# the codex-tmux skill's 90s; keep a hard ceiling so a hung turn cannot stall forever.
CS_POLL=${CS_POLL:-2}                     # bring-up prompt poll
CS_BRINGUP_TIMEOUT=${CS_BRINGUP_TIMEOUT:-90}
CS_MONITOR_POLL=${CS_MONITOR_POLL:-4}     # turn-progress poll
CS_TURN_TIMEOUT=${CS_TURN_TIMEOUT:-420}
CS_LAUNCH=${CS_LAUNCH:-'codex -s workspace-write -a on-request'}

cs__now() { date +%s; }
cs__pane() { tmux capture-pane -t "$1" -p -S -40 2>/dev/null || true; }

# Ready = the model status line is showing and no confirm/continue prompt is active.
cs__composer_ready() {
	printf '%s' "$1" | grep -qE 'Context [0-9]+% used' || return 1
	printf '%s' "$1" | grep -qE 'Press enter to (confirm|continue|go back)|Would you like to run' && return 1
	return 0
}

# cs_bring_up SESSION WORKTREE -> launch Codex in the worktree and walk the start-up
# prompts until the composer is ready. Handles the update-available prompt (pin the
# version: "Skip until next version") and the hooks-trust prompt ("Trust all"). Any
# other blocking prompt -> fail loud rather than guess an answer to a trust gate.
cs_bring_up() {
	session=$1
	worktree=$2
	[ -n "$session" ] && [ -n "$worktree" ] || {
		echo "cs_bring_up: usage cs_bring_up SESSION WORKTREE" >&2
		return 2
	}
	tmux kill-session -t "$session" 2>/dev/null || true
	tmux new-session -d -s "$session" -c "$worktree" "$CS_LAUNCH" || {
		echo "cs_bring_up: tmux new-session failed" >&2
		return 2
	}
	deadline=$(($(cs__now) + CS_BRINGUP_TIMEOUT))
	while [ "$(cs__now)" -lt "$deadline" ]; do
		pane=$(cs__pane "$session")
		# Check readiness FIRST: composer_ready is true only when no prompt is active,
		# so this never short-circuits a real trust/update prompt. (The "Update
		# available!" BANNER lingers next to the ready composer; matching it instead of
		# the active prompt made bring-up loop forever typing into the composer.)
		if cs__composer_ready "$pane"; then
			return 0
		fi
		# Active update PROMPT only (its option line, not the residual banner): pin the
		# Codex version across the eval -> option 3, "Skip until next version".
		if printf '%s' "$pane" | grep -q 'Skip until next version'; then
			tmux send-keys -t "$session" Down Down Enter
			sleep "$CS_POLL"
			continue
		fi
		if printf '%s' "$pane" | grep -q 'Hooks need review'; then
			# Option 2: Trust all and continue (so the nudge hook can run).
			tmux send-keys -t "$session" Down Enter
			sleep "$CS_POLL"
			continue
		fi
		# An active prompt we do not recognize: do NOT auto-answer a trust gate.
		if printf '%s' "$pane" | grep -qE 'Press enter to confirm|Would you like to run|Do you (trust|want)'; then
			echo "cs_bring_up: unrecognized blocking prompt, refusing to auto-answer:" >&2
			printf '%s\n' "$pane" | tail -8 >&2
			return 3
		fi
		sleep "$CS_POLL"
	done
	echo "cs_bring_up: timed out after ${CS_BRINGUP_TIMEOUT}s waiting for composer" >&2
	printf '%s\n' "$(cs__pane "$session")" | tail -8 >&2
	return 3
}

# cs_dispatch SESSION BRIEF_FILE -> paste the brief and submit it. load-buffer +
# paste-buffer (not send-keys) so a long brief is not garbled; the sleep lets the
# async paste land before Enter. Verify the turn actually started.
cs_dispatch() {
	session=$1
	brief=$2
	[ -f "$brief" ] || {
		echo "cs_dispatch: brief not found: $brief" >&2
		return 2
	}
	tmux load-buffer "$brief" || {
		echo "cs_dispatch: load-buffer failed" >&2
		return 2
	}
	tmux paste-buffer -t "$session"
	sleep 1
	tmux send-keys -t "$session" Enter
	i=0
	while [ "$i" -lt 6 ]; do
		sleep "$CS_POLL"
		pane=$(cs__pane "$session")
		printf '%s' "$pane" | grep -qE 'Working \(|esc to interrupt|Worked for ' && return 0
		i=$((i + 1))
	done
	echo "cs_dispatch: turn did not start (no Working indicator)" >&2
	return 3
}

# cs_monitor SESSION [TIMEOUT] -> wait for the turn to end; echo a status word:
#   DONE     the turn finished (active indicator gone after the turn was in flight)
#   DEVIATED Codex tried a gated non-edit command (cargo/git/network); we cancelled it
#   TIMEOUT  hard ceiling hit
# Always returns 0; the caller branches on the word.
#
# cs_dispatch has already confirmed the turn STARTED, so a turn is in flight. Codex
# shows "esc to interrupt" continuously while working and drops it the instant the turn
# ends; two consecutive polls without it = turn over. (The "Worked for" rule is
# unreliable — it scrolls out of the capture window on fast single-edit turns.)
cs_monitor() {
	session=$1
	timeout=${2:-$CS_TURN_TIMEOUT}
	deadline=$(($(cs__now) + timeout))
	gone=0
	while [ "$(cs__now)" -lt "$deadline" ]; do
		pane=$(cs__pane "$session")
		# Approval escalation: the eval briefs forbid non-edit commands (and the commit
		# case runs -a never, so it never gates). Cancel and flag rather than approve or
		# hang. Check before the gone-counter: a waiting approval also lacks the active
		# indicator and would otherwise read as DONE.
		if printf '%s' "$pane" | grep -qE 'Would you like to run|Press enter to confirm or esc to cancel'; then
			tmux send-keys -t "$session" Escape
			sleep 1
			echo DEVIATED
			return 0
		fi
		if printf '%s' "$pane" | grep -qE 'Working \(|esc to interrupt'; then
			gone=0
		else
			gone=$((gone + 1))
			if [ "$gone" -ge 2 ]; then
				echo DONE
				return 0
			fi
		fi
		sleep "$CS_MONITOR_POLL"
	done
	echo TIMEOUT
	return 0
}

# cs_teardown SESSION -> kill the tmux session (worktree cleanup is the caller's job).
cs_teardown() {
	tmux kill-session -t "$1" 2>/dev/null || true
}
