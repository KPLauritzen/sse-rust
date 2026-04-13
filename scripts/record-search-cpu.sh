#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SCRIPT_PATH="${SCRIPT_DIR}/$(basename "${BASH_SOURCE[0]}")"
TMP_DIR="${REPO_ROOT}/tmp"
STATE_DIR="${TMP_DIR}/record-search-cpu"
PID_FILE="${STATE_DIR}/sampler.pid"
LATEST_FILE="${STATE_DIR}/latest-dir"
DEFAULT_INTERVAL=1
DEFAULT_PATTERN='cargo|rustc|search'

usage() {
  cat <<'EOF'
Usage:
  scripts/record-search-cpu.sh start [label] [interval_seconds]
  scripts/record-search-cpu.sh stop
  scripts/record-search-cpu.sh status

Behavior:
  - `start` begins a background sampler on the host side and writes logs under
    `tmp/search-cpu-<timestamp>-<label>/`.
  - `stop` stops the current sampler.
  - `status` prints whether a sampler is running and where it is logging.

Recorded artifacts:
  - `meta.txt`            static environment metadata
  - `processes.log`       timestamped process snapshots for cargo/rustc/search
  - `search-threads.log`  per-thread snapshots for live `search` processes
  - `top-threads.log`     `top -H` snapshots for live `search` processes
  - `sampler.log`         sampler lifecycle messages

Examples:
  scripts/record-search-cpu.sh start k4-host-probe
  scripts/record-search-cpu.sh status
  scripts/record-search-cpu.sh stop
EOF
}

die() {
  echo "$*" >&2
  exit 1
}

ensure_state_dir() {
  mkdir -p "${STATE_DIR}"
}

is_pid_running() {
  local pid="$1"
  [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null
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

write_metadata() {
  local outdir="$1"
  {
    echo "started_at=$(date -Is)"
    echo "repo_root=${REPO_ROOT}"
    echo "host=$(hostname)"
    echo "kernel=$(uname -a)"
    echo "nproc=$(nproc)"
    echo "pattern=${DEFAULT_PATTERN}"
  } > "${outdir}/meta.txt"
}

sample_loop() {
  local outdir="$1"
  local interval="$2"
  local child_pid_file="${3:-}"

  mkdir -p "${outdir}"
  write_metadata "${outdir}"
  if [[ -n "${child_pid_file}" ]]; then
    printf '%s\n' "$$" > "${child_pid_file}"
  fi
  {
    echo "sampler_started_at=$(date -Is)"
    echo "sampler_pid=$$"
    echo "interval_seconds=${interval}"
  } >> "${outdir}/sampler.log"
  trap 'echo "sampler_exited_at=$(date -Is)" >> "${outdir}/sampler.log"' EXIT

  while true; do
    local timestamp
    timestamp="$(date -Is)"

    {
      echo "=== ${timestamp} ==="
      ps -eo pid,ppid,pgid,psr,pcpu,pmem,nlwp,etime,stat,comm,args --sort=pid \
        | awk 'NR==1 || /cargo|rustc|search/ {print}'
      echo
    } >> "${outdir}/processes.log"

    mapfile -t search_pids < <(pgrep -x search || true)
    if (( ${#search_pids[@]} > 0 )); then
      for pid in "${search_pids[@]}"; do
        {
          echo "=== ${timestamp} pid=${pid} ==="
          ps -L -p "${pid}" -o pid,tid,psr,pcpu,pmem,stat,comm
          echo
        } >> "${outdir}/search-threads.log"

        {
          echo "=== ${timestamp} pid=${pid} ==="
          top -b -n 1 -H -p "${pid}" | sed -n '1,22p'
          echo
        } >> "${outdir}/top-threads.log" 2>&1 || true
      done
    fi

    sleep "${interval}"
  done
}

start_sampler() {
  ensure_state_dir

  if [[ -f "${PID_FILE}" ]]; then
    local current_pid
    current_pid="$(cat "${PID_FILE}")"
    if is_pid_running "${current_pid}"; then
      die "sampler already running with pid ${current_pid}"
    fi
    rm -f "${PID_FILE}"
  fi

  local label interval stamp outdir pid
  label="$(sanitize_label "${1:-probe}")"
  interval="${2:-${DEFAULT_INTERVAL}}"
  [[ "${interval}" =~ ^[0-9]+$ ]] || die "interval_seconds must be a non-negative integer"
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  outdir="${TMP_DIR}/search-cpu-${stamp}-${label}"
  mkdir -p "${outdir}"

  local child_pid_file
  child_pid_file="${outdir}/child.pid"

  if command -v setsid >/dev/null 2>&1; then
    setsid -f "${SCRIPT_PATH}" __sample-loop "${outdir}" "${interval}" "${child_pid_file}" \
      > /dev/null 2>&1 < /dev/null
  else
    nohup "${SCRIPT_PATH}" __sample-loop "${outdir}" "${interval}" "${child_pid_file}" \
      > /dev/null 2>&1 < /dev/null &
  fi

  for _ in $(seq 1 20); do
    if [[ -f "${child_pid_file}" ]]; then
      pid="$(cat "${child_pid_file}")"
      break
    fi
    sleep 0.1
  done

  [[ -n "${pid:-}" ]] || die "failed to start sampler"
  rm -f "${child_pid_file}"

  echo "${pid}" > "${PID_FILE}"
  echo "${outdir}" > "${LATEST_FILE}"

  echo "started sampler pid=${pid}"
  echo "output_dir=${outdir}"
}

stop_sampler() {
  ensure_state_dir

  [[ -f "${PID_FILE}" ]] || die "no sampler pid file found"

  local pid
  pid="$(cat "${PID_FILE}")"
  if ! is_pid_running "${pid}"; then
    rm -f "${PID_FILE}"
    echo "sampler pid ${pid} was already stopped"
    [[ -f "${LATEST_FILE}" ]] && echo "output_dir=$(cat "${LATEST_FILE}")"
    return 0
  fi

  kill "${pid}"
  for _ in $(seq 1 20); do
    if ! is_pid_running "${pid}"; then
      break
    fi
    sleep 0.1
  done
  rm -f "${PID_FILE}"

  echo "stopped sampler pid=${pid}"
  if [[ -f "${LATEST_FILE}" ]]; then
    echo "output_dir=$(cat "${LATEST_FILE}")"
  fi
}

status_sampler() {
  ensure_state_dir

  if [[ ! -f "${PID_FILE}" ]]; then
    echo "status=stopped"
    [[ -f "${LATEST_FILE}" ]] && echo "latest_dir=$(cat "${LATEST_FILE}")"
    return 0
  fi

  local pid
  pid="$(cat "${PID_FILE}")"
  if is_pid_running "${pid}"; then
    echo "status=running"
    echo "pid=${pid}"
    [[ -f "${LATEST_FILE}" ]] && echo "output_dir=$(cat "${LATEST_FILE}")"
  else
    echo "status=stale"
    echo "pid=${pid}"
    [[ -f "${LATEST_FILE}" ]] && echo "latest_dir=$(cat "${LATEST_FILE}")"
  fi
}

cmd="${1:-}"
case "${cmd}" in
  start)
    start_sampler "${2:-probe}" "${3:-${DEFAULT_INTERVAL}}"
    ;;
  stop)
    stop_sampler
    ;;
  status)
    status_sampler
    ;;
  __sample-loop)
    sample_loop "${2:?missing output dir}" "${3:-${DEFAULT_INTERVAL}}" "${4:-}"
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    usage >&2
    exit 1
    ;;
esac
