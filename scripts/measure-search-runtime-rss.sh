#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TMP_DIR="${REPO_ROOT}/tmp"
DEFAULT_RUNS=5
DEFAULT_KILL_AFTER="5s"

usage() {
  cat <<'EOF'
Usage:
  scripts/measure-search-runtime-rss.sh --label LABEL [options] -- COMMAND [ARGS...]

Options:
  --label LABEL       artifact label used in the tmp/ output directory
  --runs N            odd repeat count for the command (default: 5)
  --timeout LIMIT     optional timeout passed to timeout(1), e.g. 30s
  --kill-after LIMIT  follow-up timeout kill window (default: 5s)

Artifacts:
  tmp/search-measure-<timestamp>-<label>/
    meta.txt
    command.txt
    summary.tsv
    summary.txt
    run-<n>.stdout
    run-<n>.stderr
    run-<n>.metrics.tsv
    run-<n>.status

Notes:
  - Each run records wall seconds and max RSS KB from /usr/bin/time.
  - The command is run directly unless --timeout is provided.
  - For keep/reject comparisons, point COMMAND at an already-built binary such
    as target/dist/search so build noise does not contaminate the samples.

Example:
  scripts/measure-search-runtime-rss.sh \
    --label brix-ruiz-k3-gps-baseline \
    --runs 5 \
    -- target/dist/search 1,3,2,1 1,6,1,1 --max-lag 8 --max-intermediate-dim 4 --max-entry 5 --move-policy graph-plus-structured --telemetry --json
EOF
}

die() {
  echo "$*" >&2
  exit 1
}

sanitize_label() {
  local label="${1:-probe}"
  label="${label//[^A-Za-z0-9._-]/-}"
  label="${label#-}"
  label="${label%-}"
  if [[ -z "${label}" ]]; then
    label="probe"
  fi
  printf '%s\n' "${label}"
}

median_value() {
  local values=("$@")
  local count="${#values[@]}"
  local mid_index
  mid_index=$((count / 2))
  mapfile -t values < <(printf '%s\n' "${values[@]}" | LC_ALL=C sort -n)
  printf '%s\n' "${values[${mid_index}]}"
}

min_value() {
  printf '%s\n' "$@" | LC_ALL=C sort -n | head -n 1
}

max_value() {
  printf '%s\n' "$@" | LC_ALL=C sort -n | tail -n 1
}

write_metadata() {
  local outdir="$1"
  local runs="$2"
  local timeout_limit="$3"
  local kill_after="$4"
  {
    echo "started_at=$(date -Is)"
    echo "repo_root=${REPO_ROOT}"
    echo "pwd=${PWD}"
    echo "workmux_sandbox=${WORKMUX_SANDBOX:-}"
    echo "runs=${runs}"
    echo "timeout=${timeout_limit:-none}"
    echo "kill_after=${kill_after}"
    echo "hostname=$(hostname)"
    echo "kernel=$(uname -a)"
  } > "${outdir}/meta.txt"
}

label=""
runs="${DEFAULT_RUNS}"
timeout_limit=""
kill_after="${DEFAULT_KILL_AFTER}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --label)
      [[ $# -ge 2 ]] || die "--label requires a value"
      label="$2"
      shift 2
      ;;
    --runs)
      [[ $# -ge 2 ]] || die "--runs requires a value"
      runs="$2"
      shift 2
      ;;
    --timeout)
      [[ $# -ge 2 ]] || die "--timeout requires a value"
      timeout_limit="$2"
      shift 2
      ;;
    --kill-after)
      [[ $# -ge 2 ]] || die "--kill-after requires a value"
      kill_after="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[[ -n "${label}" ]] || die "--label is required"
[[ "${runs}" =~ ^[0-9]+$ ]] || die "--runs must be a positive odd integer"
(( runs > 0 )) || die "--runs must be at least 1"
(( runs % 2 == 1 )) || die "--runs must be odd so the median sample is unambiguous"
(( $# > 0 )) || die "missing command after --"

label="$(sanitize_label "${label}")"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
outdir="${TMP_DIR}/search-measure-${stamp}-${label}"
mkdir -p "${outdir}"

write_metadata "${outdir}" "${runs}" "${timeout_limit}" "${kill_after}"

{
  printf '%q ' "$@"
  printf '\n'
} > "${outdir}/command.txt"

cmd=("$@")
wall_samples=()
rss_samples=()

printf 'run\twall_seconds\tmax_rss_kb\n' > "${outdir}/summary.tsv"

for run in $(seq 1 "${runs}"); do
  run_stdout="${outdir}/run-${run}.stdout"
  run_stderr="${outdir}/run-${run}.stderr"
  run_metrics="${outdir}/run-${run}.metrics.tsv"
  run_status="${outdir}/run-${run}.status"

  set +e
  if [[ -n "${timeout_limit}" ]]; then
    /usr/bin/time -f '%e\t%M' -o "${run_metrics}" \
      timeout -k "${kill_after}" "${timeout_limit}" "${cmd[@]}" \
      > "${run_stdout}" 2> "${run_stderr}"
  else
    /usr/bin/time -f '%e\t%M' -o "${run_metrics}" \
      "${cmd[@]}" \
      > "${run_stdout}" 2> "${run_stderr}"
  fi
  status=$?
  set -e

  printf '%s\n' "${status}" > "${run_status}"
  if (( status != 0 )); then
    die "run ${run} exited with status ${status}; see ${run_stdout} and ${run_stderr}"
  fi

  IFS=$'\t' read -r wall_seconds max_rss_kb < "${run_metrics}"
  wall_samples+=("${wall_seconds}")
  rss_samples+=("${max_rss_kb}")
  printf '%s\t%s\t%s\n' "${run}" "${wall_seconds}" "${max_rss_kb}" >> "${outdir}/summary.tsv"
done

median_wall="$(median_value "${wall_samples[@]}")"
median_rss="$(median_value "${rss_samples[@]}")"
min_wall="$(min_value "${wall_samples[@]}")"
max_wall="$(max_value "${wall_samples[@]}")"
min_rss="$(min_value "${rss_samples[@]}")"
max_rss="$(max_value "${rss_samples[@]}")"

{
  echo "output_dir=${outdir}"
  echo "label=${label}"
  echo "runs=${runs}"
  echo "median_wall_seconds=${median_wall}"
  echo "min_wall_seconds=${min_wall}"
  echo "max_wall_seconds=${max_wall}"
  echo "median_max_rss_kb=${median_rss}"
  echo "min_max_rss_kb=${min_rss}"
  echo "max_max_rss_kb=${max_rss}"
  printf 'wall_samples_seconds=%s\n' "$(IFS=,; echo "${wall_samples[*]}")"
  printf 'max_rss_samples_kb=%s\n' "$(IFS=,; echo "${rss_samples[*]}")"
} > "${outdir}/summary.txt"

cat "${outdir}/summary.txt"
