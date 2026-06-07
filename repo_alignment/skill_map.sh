# shellcheck shell=sh
# DATA for the repo-alignment hooks: which skill-area a file belongs to, and the terse
# factual reminder for that area. Editing a nudge = editing this file. No logic here.
#
# Reminder wording is FACTUAL + a skill pointer, light emphasis, < ~300 chars, one area.
# Imperative / "SYSTEM:" phrasing can trip prompt-injection defenses (the model surfaces
# the text instead of acting on it), so we state facts and point at the skill.

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
		echo "Rust file. thiserror at the lib boundary; closed enums with exhaustive matches (no catch-all _); borrow over clone; clippy -D warnings. See /rust-coding and /source-code-organization."
		;;
	python)
		echo "Python file. uv/ruff/ty workflow; Pydantic v2 strict+frozen at boundaries; package by concept, no utils/helpers/manager. See /python-dev-tooling, /pydantic-models, /source-code-organization."
		;;
	*) echo "" ;;
	esac
}
