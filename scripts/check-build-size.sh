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
# Exit:  0 = within budget, 1 = over budget (or disk critically low).

set -uo pipefail

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
declare -a NAMES SIZES OVER
TOTAL_KB=0

while IFS= read -r d; do
  [ -d "$d" ] || continue
  kb=$(du -sk "$d" 2>/dev/null | awk '{print $1}')
  [ -z "$kb" ] && continue
  NAMES+=("$d")
  SIZES+=("$kb")
  TOTAL_KB=$((TOTAL_KB + kb))
done < <(find . -maxdepth 4 -type d -name target \
           \( -path './target' -o -path './.claude/worktrees/*/target' \) 2>/dev/null)

FREE_GB=$(df -BG --output=avail "$REPO_ROOT" 2>/dev/null | tail -1 | tr -dc '0-9')
FREE_GB=${FREE_GB:-0}

say "== build size guard =="
if [ "${#NAMES[@]}" -eq 0 ]; then
  say "${GREEN}ok:   no target trees present${NC}"
else
  for i in "${!NAMES[@]}"; do
    sz=$(gb "${SIZES[$i]}")
    if [ "$sz" -gt "$TARGET_BUDGET_GB" ]; then
      say "${RED}  ${NAMES[$i]}: ${sz} GiB  (budget ${TARGET_BUDGET_GB} GiB)${NC}"
      OVER+=("${NAMES[$i]} (${sz} GiB)")
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

if [ "$TOTAL_GB" -gt "$TOTAL_BUDGET_GB" ]; then
  say "${RED}FAIL: build trees total ${TOTAL_GB} GiB, over the ${TOTAL_BUDGET_GB} GiB budget.${NC}"
  say "${YELLOW}  Reclaim:  rm -rf <tree>   (regenerable; costs one rebuild)${NC}"
  say "${YELLOW}  Worktree trees are full duplicates — clear those first.${NC}"
  STATUS=1
fi

if [ "$FREE_GB" -lt "$MIN_FREE_GB" ]; then
  say "${RED}FAIL: only ${FREE_GB} GiB free (need ${MIN_FREE_GB} GiB to build safely).${NC}"
  say "${YELLOW}  Below this, a build can pin I/O and make the machine unusable.${NC}"
  STATUS=1
fi

if [ "$STATUS" -eq 0 ]; then
  say "${GREEN}PASS: build trees within budget.${NC}"
fi

exit "$STATUS"
