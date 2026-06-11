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

# reminder_for_area AREA -> echoes the one-line reminder for an area (empty if none).
reminder_for_area() {
	case "$1" in
	rust)
		echo "Editing a Rust file. Relevant skills: /rust-coding, /source-code-organization."
		;;
	python)
		echo "Editing a Python file. Relevant skills: /python-dev-tooling, /source-code-organization (and /pydantic-models if you're defining data models)."
		;;
	*) echo "" ;;
	esac
}
