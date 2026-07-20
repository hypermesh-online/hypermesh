#!/usr/bin/env bash
# Written by Richard Christopher, Copyright 2026 Hypermesh Foundation
#
# check-no-pos-magnitude.sh — CI guard for the canonical Proof of State model.
#
# CANONICAL MODEL
#   PoStake = AUTHORIZATION (WHO)  — binds an identity. NEVER a magnitude.
#   PoWork  = the HASH of work done (WHAT) — `work_hash`. NEVER difficulty/nonce/mining.
#   PoSpace = LOCATION (WHERE)     — binds node_id + storage_path. Capacity is
#                                    DESCRIPTIVE and must NEVER gate admission.
#   PoTime  = WHEN                 — the only legitimate quantitative bound
#                                    (clock freshness).
#
# WHY THIS SCRIPT EXISTS
#   Three consecutive cleanup passes missed violations. Every single miss was
#   either (a) in a file the compiler never reads — an orphaned/undeclared
#   module, an unregistered directory, a feature-gated test — or (b) a magnitude
#   that had been RENAMED rather than removed. So this script greps the ENTIRE
#   worktree by BEHAVIOUR and SHAPE, not by identifier, and deliberately does
#   NOT limit itself to src/ or to files that compile.
#
# Usage: scripts/check-no-pos-magnitude.sh
# Exit:  0 = clean, 1 = violations found.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
VIOLATIONS=0

# ---------------------------------------------------------------------------
# ALLOWLIST — genuine exceptions, each with its justification.
#
# These are NOT Proof of State paths. They are unrelated domains that happen to
# use arithmetic or the word "stake". Every entry must state WHY it is legitimate.
# ---------------------------------------------------------------------------
#
#   caesar/src/                     Caesar wallet / token economy. A separate,
#                                   legitimate economic system. Token balances
#                                   and staking amounts are its actual domain —
#                                   they are money, not proofs, and never gate
#                                   PoS admission.
#
#   catalog/src/distribution/dht/   Kademlia XOR-distance bucket indexing.
#                                   Numeric comparisons here are routing-table
#                                   maths, unrelated to proof validation.
#
#   catalog/src/validation/         Halstead complexity metrics. "difficulty" is
#                                   a source-code-complexity measure, not PoW.
#
#   */geospatial*, */tensor*        Divide-by-zero and magnitude guards in
#   */matrix/*                      coordinate maths. Not validation paths.
#
#   lib/src/proof.rs                Home of the canonical types. Contains the
#                                   `total_size <= total_storage` SELF-CONSISTENCY
#                                   check — that is an internal sanity invariant
#                                   (you cannot store more than you have), not an
#                                   admission gate against a constant.
#
#   scripts/check-no-pos-magnitude.sh  This file names the forbidden patterns in
#                                   order to detect them.
#
#   target/, .git/                  Build artefacts and VCS internals.
#
ALLOW_RE='^(\./)?(caesar/src/|catalog/src/distribution/dht/|catalog/src/validation/|lib/src/proof\.rs|scripts/check-no-pos-magnitude\.sh|target/|\.git/)'
ALLOW_PATH_RE='(geospatial|tensor|/matrix/)'

# Collect candidate files: the WHOLE worktree, every .rs and .c file, including
# orphaned/undeclared modules, tests, examples and other-crate directories.
candidates() {
  find . \
    \( -path ./target -o -path ./.git -o -path './*/target' \
       -o -path ./.claude/worktrees -o -path './.claude/worktrees/*' \
       -o -name node_modules -o -name vendor \) -prune -o \
    \( -name '*.rs' -o -name '*.c' -o -name '*.h' \) -print
}

is_allowed() {
  local f="$1"
  [[ "$f" =~ $ALLOW_RE ]] && return 0
  [[ "$f" =~ $ALLOW_PATH_RE ]] && return 0
  return 1
}

report() {
  local label="$1"; shift
  local hits="$1"
  if [ -n "$hits" ]; then
    echo -e "${RED}FAIL: ${label}${NC}"
    echo "$hits" | sed 's/^/    /'
    VIOLATIONS=$((VIOLATIONS + 1))
  else
    echo -e "${GREEN}ok:   ${label}${NC}"
  fi
}

FILES="$(candidates | while read -r f; do is_allowed "$f" || echo "$f"; done)"

# ---------------------------------------------------------------------------
# NARROW, DOCUMENTED EXCEPTIONS (file + identifier scoped, never whole-file).
#
# Kept deliberately narrow so these files remain scanned for every OTHER
# violation class — a whole-file allowlist would recreate the blind spot this
# script exists to close.
#
#   blockmatrix/src/blockchain/chain.rs : total_size
#       Blockchain byte accounting (`chain.get_total_size()`) — the summed size
#       of persisted blocks. Storage bookkeeping, NOT a PoSpace proof field and
#       not on any admission path.
#
#   hypermesh-ebpf/programs/hypermesh_xdp.c : difficulty
#       Appears only in a comment marking the wire field that USED to carry PoW
#       difficulty and is now a zeroed reserved word. Documents the removal.
#
#   hypermesh-ebpf/src/{validation,hypermesh_headers}.rs : difficulty
#       Assertion messages in guard tests that PROVE difficulty is absent.
#
#   stoq/examples/pos_integration_demo.rs : difficulty
#       Demo output string stating PoWork is a hash, not a difficulty.
# ---------------------------------------------------------------------------
EXCEPTIONS_RE='^\./(blockmatrix/src/blockchain/chain\.rs:[0-9]+:.*total_size|hypermesh-ebpf/(programs/hypermesh_xdp\.c|src/(validation|hypermesh_headers)\.rs):[0-9]+:.*difficult|stoq/examples/pos_integration_demo\.rs:[0-9]+:.*difficult)'

# Match on CODE SHAPE, not prose: blank out string literals and strip comments
# before testing, so assertion messages and doc comments that NAME a forbidden
# concept in order to forbid it do not trip the guard.
#
# NOTE: awk uses POSIX ERE — it does NOT support \s or \b. Using them here would
# silently match nothing and turn this guard into a permanent false PASS (that
# bug was caught by the self-test below). Word boundaries are therefore spelled
# out as [^A-Za-z0-9_] and whitespace as [[:space:]].
B_OPEN='(^|[^A-Za-z0-9_])'
B_CLOSE='([^A-Za-z0-9_]|$)'

scan() {
  local pattern="$1"
  echo "$FILES" | tr '\n' '\0' | xargs -0 -r awk -v pat="$pattern" '
    {
      line = $0
      sub(/\/\/.*$/, "", line)          # strip // comments
      sub(/^[ \t]*\*.*$/, "", line)     # strip /* */ continuation lines
      gsub(/"[^"]*"/, "\"\"", line)     # blank string literal contents
      if (line ~ pat) printf "%s:%d:%s\n", FILENAME, FNR, $0
    }
  ' 2>/dev/null | grep -vE "$EXCEPTIONS_RE" || true
}

# Self-test: prove the matcher actually matches before trusting a PASS.
# A guard that reads nothing is worse than no guard at all.
selftest() {
  local probe="/tmp/pos-magnitude-selftest-$$.rs"
  printf 'fn f() {\n    if proof.space_proof.total_size == 0 {\n        let stake_amount = 5;\n    }\n}\n' > "$probe"
  local hits
  hits="$(awk -v pat="${B_OPEN}(total_size|capacity)[[:space:]]*(==|!=|<|>|<=|>=)[[:space:]]*[0-9]" '
    { line=$0; if (line ~ pat) print FILENAME ":" FNR }' "$probe")"
  rm -f "$probe"
  if [ -z "$hits" ]; then
    echo -e "${RED}FATAL: guard self-test failed — the matcher detects nothing.${NC}" >&2
    echo -e "${RED}Refusing to report PASS. Fix the regex engine assumptions.${NC}" >&2
    exit 2
  fi
}
selftest

echo "== Proof of State magnitude guard =="
echo "   scanning $(echo "$FILES" | grep -c . ) files (whole worktree, incl. orphaned/undeclared)"
echo

# 1. PoStake must never carry an amount. `stake_amount` is forbidden outright.
report "no 'stake_amount' (PoStake is authorization, never a magnitude)" \
  "$(scan "${B_OPEN}stake_amount${B_CLOSE}")"

# 2. Renamed-not-removed stake magnitudes. Renaming a magnitude does not make it
#    an authorization — this class is why earlier passes reported false green.
report "no minimum/threshold stake fields (renamed magnitudes)" \
  "$(scan "${B_OPEN}(minimum_stake|min_stake|min_validation_stake|stake_threshold|min_stake_amount|stake_requirement)${B_CLOSE}")"

# 3. Arbitrary sanity ceilings that smuggle magnitude back in.
report "no MAX_REASONABLE_* magnitude ceilings" \
  "$(scan "${B_OPEN}MAX_REASONABLE_[A-Z_]+")"

# 4. Capacity compared against a constant. This is the defect class that
#    survived three passes: `total_size == 0` denies admission to every
#    freshly-provisioned node. PoSpace answers WHERE, not how-much.
report "no capacity/compute compared against a constant (capacity never gates)" \
  "$(scan "${B_OPEN}(total_size|total_storage|capacity|capacity_commitment|computational_power|compute_power|minimum_compute_power|min_space_commitment)[[:space:]]*(==|!=|<|>|<=|>=)[[:space:]]*[0-9]")"

# 5. PoWork is the HASH of work done, not mining.
#    NOTE: a bare `nonce` is NOT forbidden — TimeProof and WireSignedProof use
#    anti-replay nonces, which are legitimate and unrelated to mining. Only a
#    nonce attached to the WORK proof (the mining shape) is a violation.
report "no difficulty/mining fields on PoS paths" \
  "$(scan "${B_OPEN}(difficulty_target|min_work_difficulty|work_difficulty|difficulty)${B_CLOSE}")"

report "no mining nonce on the work proof" \
  "$(scan "work_proof[.a-z_]*nonce|${B_OPEN}PoWorkProof${B_CLOSE}")"

# 6. Duplicate PoS type definitions outside lib. There is ONE canonical type set
#    and it lives in lib/src/proof.rs.
DUP="$(echo "$FILES" | grep -v '^\./lib/' | tr '\n' '\0' \
  | xargs -0 -r grep -nEH '^[[:space:]]*pub struct (FourProof|PoSpaceProof|PoStakeProof|PoWorkProof|PoTimeProof|SpaceProof|StakeProof|WorkProof|TimeProof|StateProof|ProofOfState)[[:space:]{(]' 2>/dev/null || true)"
report "no duplicate PoS type definitions outside lib/" "$DUP"

echo
if [ "$VIOLATIONS" -eq 0 ]; then
  echo -e "${GREEN}PASS: no Proof of State magnitude violations.${NC}"
  exit 0
fi
echo -e "${RED}FAILED: $VIOLATIONS violation class(es).${NC}"
echo -e "${YELLOW}PoStake = WHO. PoWork = hash of work. PoSpace = WHERE. PoTime = WHEN.${NC}"
echo -e "${YELLOW}No proof carries a magnitude; no magnitude gates admission.${NC}"
exit 1
