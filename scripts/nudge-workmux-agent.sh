#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/nudge-workmux-agent.sh <handle> [iterations] [sleep_seconds] [message...]
       scripts/nudge-workmux-agent.sh <handle> [iterations] [sleep_seconds] -f|--file <message_file>

Defaults:
  iterations:    12
  sleep_seconds: 600
  message:       a multiline continue-working instruction

Example:
  scripts/nudge-workmux-agent.sh optimize-program-md-longrun
  scripts/nudge-workmux-agent.sh optimize-program-md-longrun 6 300 "continue working"
  scripts/nudge-workmux-agent.sh main 12 600 'Line one.\nLine two.'
  scripts/nudge-workmux-agent.sh main 12 600 --file tmp/followup.md
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
scratch_dir="$repo_root/tmp"

handle="${1:-}"
iterations="${2:-12}"
sleep_seconds="${3:-600}"
shift $(( $# >= 3 ? 3 : $# ))

if (( $# >= 2 )) && [[ "${1:-}" == "-f" || "${1:-}" == "--file" ]]; then
  message_file="${2:-}"
  if [[ -z "$message_file" ]]; then
    echo "--file requires a path" >&2
    exit 1
  fi
  if [[ ! -f "$message_file" ]]; then
    echo "message file not found: $message_file" >&2
    exit 1
  fi
  raw_message="$(<"$message_file")"
elif (( $# > 0 )); then
  raw_message="$*"
else
  raw_message=$'Continue working.\nIf you think you are done, pick the next highest-leverage optimization step from research/program.md and keep going without waiting for user input.\nPrefer profiling and measurement first so you cut in the right place.'
fi

# Callers often quote \n literally; decode it so file-backed sends stay multiline.
message="${raw_message//\\n/$'\n'}"

# Codex panes only submit reliably when the file payload contains a real line break.
if [[ "$message" != *$'\n'* ]]; then
  message+=$'\n'
fi
if [[ "$message" != *$'\n' ]]; then
  message+=$'\n'
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
    echo "[$timestamp] iteration $i/$iterations: '$handle' is $status_line, sending a message"
    mkdir -p "$scratch_dir"
    tmpfile="$(mktemp "$scratch_dir/workmux-nudge.XXXXXX.txt")"
    trap 'rm -f "$tmpfile"' EXIT
    printf '%s' "$message" > "$tmpfile"
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
