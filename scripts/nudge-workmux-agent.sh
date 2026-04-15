#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/nudge-workmux-agent.sh <handle> [iterations] [sleep_seconds] [message...]

Defaults:
  iterations:    12
  sleep_seconds: 600
  message:       a multiline continue-working instruction

Example:
  scripts/nudge-workmux-agent.sh optimize-program-md-longrun
  scripts/nudge-workmux-agent.sh optimize-program-md-longrun 6 300 "continue working"
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

handle="${1:-}"
iterations="${2:-12}"
sleep_seconds="${3:-600}"
shift $(( $# >= 3 ? 3 : $# ))
if (( $# > 0 )); then
  message="$*"
else
  message=$'Continue working.\nIf you think you are done, pick the next highest-leverage optimization step from research/program.md and keep going without waiting for user input.\nPrefer profiling and measurement first so you cut in the right place.'
fi

if [[ -z "$handle" ]]; then
  usage >&2
  exit 1
fi

if ! [[ "$iterations" =~ ^[0-9]+$ ]] || ! [[ "$sleep_seconds" =~ ^[0-9]+$ ]]; then
  echo "iterations and sleep_seconds must be non-negative integers" >&2
  exit 1
fi

for ((i = 1; i <= iterations; i++)); do
  timestamp="$(date '+%Y-%m-%d %H:%M:%S')"
  status_line="$(workmux status "$handle" --json | jq -r '.[0].status // empty')"

  if [[ -z "$status_line" ]]; then
    echo "[$timestamp] iteration $i/$iterations: could not determine status for '$handle'" >&2
  elif [[ "$status_line" == "waiting" || "$status_line" == "idle" || "$status_line" == "done" ]]; then
    echo "[$timestamp] iteration $i/$iterations: '$handle' is $status_line, sending: $message"
    tmpfile="$(mktemp)"
    trap 'rm -f "$tmpfile"' EXIT
    printf '%s\n' "$message" > "$tmpfile"
    workmux send "$handle" -f "$tmpfile"
    rm -f "$tmpfile"
    trap - EXIT
  else
    echo "[$timestamp] iteration $i/$iterations: '$handle' is $status_line, no action"
  fi

  if (( i < iterations )); then
    sleep "$sleep_seconds"
  fi
done
