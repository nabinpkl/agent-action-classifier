# shellcheck shell=sh
# DATA for the repo-alignment hooks: which skill-area a file belongs to, and the
# attention-pointer for that area. Editing a nudge = editing this file. No logic here.
#
# The reminder is a soft ATTENTION SHIFT to the relevant skills, NOT a prescription:
# it names the area and points at the skills, and lets the skill be the source of truth.
# It must NOT bake in specific technical mandates (no "use thiserror", no "Pydantic
# strict+frozen") — those duplicate the skill, freeze a snapshot that goes stale, and
# can push a wrong design where it does not apply. Imperative / "SYSTEM:" phrasing can
# also trip prompt-injection defenses, so keep it a plain pointer.

# path_to_area PATH -> echoes the skill-area for a file, or nothing if none applies.
path_to_area() {
	case "$1" in
	*.rs) echo rust ;;
	*.py) echo python ;;
	*) echo "" ;;
	esac
}

# skills_for_area AREA -> echoes the relevant skills for an area, space-separated (empty
# if none). THE single source of truth for the area->skills map: both the pre-edit
# reminder (below) and the Stop review read it, so the two hooks cannot drift.
skills_for_area() {
	case "$1" in
	rust) echo "/rust-coding /source-code-organization" ;;
	python) echo "/python-dev-tooling /source-code-organization" ;;
	*) echo "" ;;
	esac
}

# reminder_for_area AREA -> the one-line pre-edit attention pointer (empty if none). The
# skill names come from skills_for_area so the map lives in exactly one place; only the
# per-area prose (the label, and the conditional pydantic note) is local here.
reminder_for_area() {
	case "$1" in
	rust)
		echo "Editing a Rust file. Relevant skills: $(skills_for_area rust | sed 's/ /, /g')."
		;;
	python)
		echo "Editing a Python file. Relevant skills: $(skills_for_area python | sed 's/ /, /g') (and /pydantic-models if you're defining data models)."
		;;
	*) echo "" ;;
	esac
}
