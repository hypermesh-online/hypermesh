#!/usr/bin/env bash
# Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
#
# check-build-size.sh — guard against the build tree eating the disk.
#
# WHY THIS EXISTS
#   On 2026-07-22 the workspace's target trees reached 354 GB (210 GB in
#   core/target, 144 GB in a single worktree) on a 1.8 TB disk that was already
#   87% full. The machine went to 98% full with I/O pressure pinned at 89%,
#   processes piled up in D-state, and the desktop became unusable — Chrome
#   would not respond because every read queued behind cargo's writes.
#
#   Root cause: ~97 integration-test binaries (61 in blockmatrix alone). Every
#   tests/*.rs file becomes its OWN binary statically linking the entire
#   dependency tree, and [profile.dev] carried `debug = true`, so full DWARF was
#   paid ~97 times over. Fixed in Cargo.toml via line-tables-only +
#   split-debuginfo + debug-free dependencies.
#
#   This script exists so the regression is caught by a number, not by a frozen
#   workstation.
#
# Usage: scripts/check-build-size.sh [--quiet]
# Exit:  0 = within budget, 1 = budget violation, 2 = inventory/measurement error.

set -uo pipefail
export LC_ALL=C

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
QUIET=0
[ "${1:-}" = "--quiet" ] && QUIET=1

# Budgets. These are deliberately generous — they catch a runaway, not normal
# growth. A healthy full workspace build with the current profile lands well
# under TARGET_BUDGET_GB.
TARGET_BUDGET_GB=80        # a single target/ tree
TOTAL_BUDGET_GB=140        # all target trees, including worktrees
MIN_FREE_GB=60             # free space below which building is unsafe

say() { [ "$QUIET" -eq 1 ] || echo -e "$@"; }

gb() { echo $(( ${1:-0} / 1048576 )); }   # du -k output -> GiB

# ---------------------------------------------------------------------------
# Collect every target tree: the main one plus any under .claude/worktrees/.
# Worktrees are the silent multiplier — each carries a full duplicate.
# ---------------------------------------------------------------------------
fatal() { printf 'FATAL: %s\n' "$*" >&2; exit 2; }

valid_kib() {
  local value="$1"
  [[ "$value" =~ ^(0|[1-9][0-9]*)$ ]] || return 1
  [ "${#value}" -lt 19 ] || {
    [ "${#value}" -eq 19 ] && [[ "$value" < 9223372036854775808 ]]
  }
}

measure_target() {
  local output value
  output=$(du -sk "$1") || fatal "du failed for $1"
  value=${output%%$'\t'*}
  [[ "$output" == "$value"$'\t'"$1" ]] && valid_kib "$value" ||
    fatal "invalid or missing du measurement for $1"
  printf '%s\n' "$value"
}

available_kib() {
  local output header value
  output=$(df -k --output=avail "$REPO_ROOT") || fatal "df failed"
  [[ "$output" == *$'\n'* ]] || fatal "missing df measurement"
  header=${output%%$'\n'*}
  value=${output#*$'\n'}
  [[ "$header" =~ ^[[:blank:]]*Avail[[:blank:]]*$ ]] || fatal "invalid df measurement header"
  [[ "$value" =~ ^[[:blank:]]*([0-9]+)[[:blank:]]*$ ]] || fatal "invalid df measurement"
  value=${BASH_REMATCH[1]}
  valid_kib "$value" || fatal "invalid df measurement"
  printf '%s\n' "$value"
}

collect_sizes() {
  local pid d kb
  local -a inventory=()
  mapfile -d '' -t inventory < <(find . -maxdepth 4 -type d -name target \
    \( -path './target' -o -path './.claude/worktrees/*/target' \) -print0) ||
    fatal "cannot read build inventory"
  # Process-substitution failure is separate from mapfile success.
  pid=$!
  wait "$pid" || fatal "cannot enumerate build inventory"
  for d in "${inventory[@]}"; do
    [ -d "$d" ] || fatal "build inventory path disappeared: $d"
    kb=$(measure_target "$d") || exit 2
    [ "$kb" -le "$((9223372036854775807 - TOTAL_KB))" ] || fatal "total measurement overflow"
    NAMES+=("$d")
    SIZES+=("$kb")
    TOTAL_KB=$((TOTAL_KB + kb))
  done
}

declare -a NAMES=() SIZES=() OVER=()
TOTAL_KB=0
collect_sizes
FREE_KB=$(available_kib) || exit 2
FREE_GB=$(gb "$FREE_KB")
TARGET_BUDGET_KB=$((TARGET_BUDGET_GB * 1048576))
TOTAL_BUDGET_KB=$((TOTAL_BUDGET_GB * 1048576))
MIN_FREE_KB=$((MIN_FREE_GB * 1048576))

say "== build size guard =="
if [ "${#NAMES[@]}" -eq 0 ]; then
  say "${GREEN}ok:   no target trees present${NC}"
else
  for i in "${!NAMES[@]}"; do
    sz=$(gb "${SIZES[$i]}")
    if [ "${SIZES[$i]}" -gt "$TARGET_BUDGET_KB" ]; then
      say "${RED}  ${NAMES[$i]}: ${sz} GiB  (budget ${TARGET_BUDGET_GB} GiB)${NC}"
      OVER+=("${NAMES[$i]} (${SIZES[$i]} KiB)")
    else
      say "  ${NAMES[$i]}: ${sz} GiB"
    fi
  done
fi

TOTAL_GB=$(gb "$TOTAL_KB")
say "  ----"
say "  total: ${TOTAL_GB} GiB across ${#NAMES[@]} tree(s); ${FREE_GB} GiB free on disk"
say ""

STATUS=0

# A single oversized tree is a failure on its own. Reporting it in red and then
# exiting 0 because the TOTAL happened to fit is a guard that lies — the exact
# false-green shape that made check-no-pos-magnitude.sh useless until it grew a
# self-test. Anything printed as a violation must set the exit status.
if [ "${#OVER[@]}" -gt 0 ]; then
  say "${RED}FAIL: ${#OVER[@]} tree(s) over the ${TARGET_BUDGET_GB} GiB per-tree budget:${NC}"
  for o in "${OVER[@]}"; do say "${RED}    $o${NC}"; done
  STATUS=1
fi

if [ "$TOTAL_KB" -gt "$TOTAL_BUDGET_KB" ]; then
  say "${RED}FAIL: build trees total ${TOTAL_KB} KiB (${TOTAL_GB} GiB), over the ${TOTAL_BUDGET_GB} GiB budget.${NC}"
  say "${YELLOW}  Reclaim:  rm -rf <tree>   (regenerable; costs one rebuild)${NC}"
  say "${YELLOW}  Worktree trees are full duplicates — clear those first.${NC}"
  STATUS=1
fi

if [ "$FREE_KB" -lt "$MIN_FREE_KB" ]; then
  say "${RED}FAIL: only ${FREE_GB} GiB free (${FREE_KB} KiB; need ${MIN_FREE_GB} GiB to build safely).${NC}"
  say "${YELLOW}  Below this, a build can pin I/O and make the machine unusable.${NC}"
  STATUS=1
fi

if [ "$STATUS" -eq 0 ]; then
  say "${GREEN}PASS: build trees within budget.${NC}"
fi

exit "$STATUS"
