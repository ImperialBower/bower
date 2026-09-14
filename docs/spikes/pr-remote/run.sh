#!/usr/bin/env bash
# EPIC-09 slice 2's experiment: what GitHub does with a branch, a pull
# request, and a merge that Bower makes by pushing main — never on the forge.
#
# Answers, on a real remote:
#   Q1  Is a PR marked MERGED when its head becomes reachable from main by a
#       plain push?
#   Q2  What happens to an open PR and a merged PR when a rebuild gives every
#       branch new SHAs and they are force-pushed?
#   Q3  (optional, needs an empty repo) Does the first branch pushed to an
#       empty repository become its default?
#
# It force-pushes. It refuses any repository but the sandbox.
#
# Usage:
#   docs/spikes/pr-remote/run.sh            # Q1 and Q2 against the sandbox
#   DRY=1 docs/spikes/pr-remote/run.sh      # build and print, push nothing
#   EMPTY_REPO=owner/name docs/spikes/pr-remote/run.sh   # also Q3
#
# Results go to docs/spikes/pr-remote/results-<run>.md.

set -euo pipefail

SANDBOX=abstecker/bower-sandbox
REPO=${REPO:-$SANDBOX}
if [ "$REPO" != "$SANDBOX" ]; then
  echo "refusing: this experiment force-pushes, and only $SANDBOX is ours to wreck" >&2
  exit 2
fi
URL=https://github.com/$REPO.git
DRY=${DRY:-}

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$HERE" rev-parse --show-toplevel)
RUN=${RUN:-$(date -u +%Y%m%d%H%M%S)}
WORK=${TMPDIR:-/tmp}/bower-pr-remote/$RUN
LOG=$HERE/results-$RUN.md
TRY=try/abandon-$RUN
FEAT=feat/merge-me-$RUN

mkdir -p "$WORK"
: > "$LOG"

say() { printf '%s\n' "$*" | tee -a "$LOG"; }
# A command that changes the remote. Under DRY=1 it is printed, not run.
remote() {
  say "    \$ $*"
  if [ -z "$DRY" ]; then "$@" 2>&1 | sed 's/^/      /' | tee -a "$LOG"; fi
}

say "# EPIC-09 slice 2 experiment — run $RUN"
say ""
say "- repository: $REPO"
say "- branches: \`$TRY\` (never merged), \`$FEAT\` (merged by a push)"
say "- dry run: ${DRY:-no}"
say ""

# Build a variant of the book: the checked-in book with @RUN@ replaced, then
# an optional sed expression applied to the chapter. Prints the repo dir.
#
# Every variant's book directory is named `book`: the directory's name is the
# book's name in every commit's `Book-Source` trailer, so variants in
# differently named directories would differ in every SHA and prove nothing.
build() {
  local name=$1 edit=${2:-}
  local book=$WORK/$name/book out=$WORK/$name/repo
  rm -rf "${WORK:?}/${name:?}"
  mkdir -p "$WORK/$name"
  cp -R "$HERE/book" "$book"
  sed -i.bak "s/@RUN@/$RUN/g" "$book/src/ch01-sandbox.md"
  if [ -n "$edit" ]; then sed -i.bak "$edit" "$book/src/ch01-sandbox.md"; fi
  rm -f "$book/src/"*.bak
  cargo run -q --manifest-path "$ROOT/Cargo.toml" -p bower -- --book "$book" build -o "$out" >&2
  echo "$out"
}

sha() { git -C "$1" rev-parse "$2^{commit}"; }

pr_state() {
  if [ -n "$DRY" ]; then echo "(dry run)"; return; fi
  gh pr view "$1" -R "$REPO" \
    --json number,state,mergedAt,headRefOid,mergeCommit \
    --jq '"#\(.number) state=\(.state) mergedAt=\(.mergedAt // "-") head=\(.headRefOid[0:7]) mergeCommit=\((.mergeCommit.oid // "-")[0:7])"'
}

# Poll a PR until it reports MERGED, for up to a minute. GitHub decides merged
# state asynchronously after a push.
wait_merged() {
  if [ -n "$DRY" ]; then echo "(dry run)"; return; fi
  local i state
  for i in $(seq 1 12); do
    state=$(gh pr view "$1" -R "$REPO" --json state --jq .state)
    if [ "$state" = MERGED ]; then echo "MERGED after ~$((i * 5))s"; return; fi
    sleep 5
  done
  echo "still $state after 60s"
}

open_pr() {
  local head=$1 title=$2 body=$3
  if [ -n "$DRY" ]; then
    say "    \$ gh pr create -R $REPO --base main --head $head --title \"$title\""
    echo "0"
    return
  fi
  gh pr create -R "$REPO" --base main --head "$head" --title "$title" --body "$body" >/dev/null
  gh pr view "$head" -R "$REPO" --json number --jq .number
}

# ---------------------------------------------------------------------------
say "## Q1 — merged by a push?"
say ""
G1=$(build one)
PRE=$(sha "$G1" step-005-main-moves)
MERGE=$(sha "$G1" step-006-merge-it)
say "Built $G1: main before the merge ${PRE:0:7}, merge ${MERGE:0:7},"
say "\`$TRY\` $(sha "$G1" "refs/heads/$TRY" | cut -c1-7), \`$FEAT\` $(sha "$G1" "refs/heads/$FEAT" | cut -c1-7)."
say ""
say "1. Push main as it stood before the merge, and both branches (main is"
say "   replaced wholesale — the sandbox's own history goes):"
remote git -C "$G1" push --force "$URL" \
  "$PRE:refs/heads/main" "refs/heads/$TRY:refs/heads/$TRY" "refs/heads/$FEAT:refs/heads/$FEAT"
say "2. Open a PR for each branch:"
PR_TRY=$(open_pr "$TRY" "An abandoned experiment" "Left open on purpose.")
PR_FEAT=$(open_pr "$FEAT" "A branch Bower merges" "Merged by a push to main, never on the forge.")
say "   - $TRY: $(pr_state "$PR_TRY")"
say "   - $FEAT: $(pr_state "$PR_FEAT")"
say "3. Push main to the merge commit — a fast-forward, no force:"
remote git -C "$G1" push "$URL" "$MERGE:refs/heads/main"
say "4. After the push:"
say "   - $FEAT: $(wait_merged "$PR_FEAT") — $(pr_state "$PR_FEAT")"
say "   - $TRY: $(pr_state "$PR_TRY")"
say ""

# ---------------------------------------------------------------------------
say "## Q2a — a rebuild that changes only the merged branch"
say ""
G2=$(build two 's/^two$/two, rebuilt/')
say "Rebuilt: \`$FEAT\` $(sha "$G1" "refs/heads/$FEAT" | cut -c1-7) → $(sha "$G2" "refs/heads/$FEAT" | cut -c1-7),"
say "merge ${MERGE:0:7} → $(sha "$G2" step-006-merge-it | cut -c1-7), \`$TRY\` $(sha "$G1" "refs/heads/$TRY" | cut -c1-7) → $(sha "$G2" "refs/heads/$TRY" | cut -c1-7) (should not move)."
remote git -C "$G2" push --force "$URL" \
  "refs/heads/$FEAT:refs/heads/$FEAT" "$(sha "$G2" step-006-merge-it):refs/heads/main"
say "After the force-push:"
say "   - $FEAT (merged): $(pr_state "$PR_FEAT")"
say "   - $TRY (open): $(pr_state "$PR_TRY")"
if [ -z "$DRY" ]; then
  say "   - every PR whose head is \`$FEAT\`:"
  gh pr list -R "$REPO" --head "$FEAT" --state all --json number,state \
    --jq '.[] | "     #\(.number) \(.state)"' | tee -a "$LOG"
fi
say ""

# ---------------------------------------------------------------------------
say "## Q2b — a rebuild that changes every SHA"
say ""
G3=$(build three 's/^base$/base, rebuilt/; s/^two$/two, rebuilt/')
say "Rebuilt from an edit to the first step: main $(sha "$G2" step-006-merge-it | cut -c1-7) → $(sha "$G3" step-006-merge-it | cut -c1-7),"
say "\`$TRY\` $(sha "$G2" "refs/heads/$TRY" | cut -c1-7) → $(sha "$G3" "refs/heads/$TRY" | cut -c1-7)."
remote git -C "$G3" push --force "$URL" \
  "refs/heads/$TRY:refs/heads/$TRY" "refs/heads/$FEAT:refs/heads/$FEAT" \
  "$(sha "$G3" step-006-merge-it):refs/heads/main"
say "After the force-push:"
say "   - $FEAT (merged): $(pr_state "$PR_FEAT")"
say "   - $TRY (open): $(pr_state "$PR_TRY")"
if [ -z "$DRY" ]; then
  say "   - $TRY commits listed on the PR:"
  gh pr view "$PR_TRY" -R "$REPO" --json commits \
    --jq '.commits[] | "     \(.oid[0:7]) \(.messageHeadline)"' | tee -a "$LOG"
fi
say ""

# ---------------------------------------------------------------------------
if [ -n "${EMPTY_REPO:-}" ]; then
  say "## Q3 — the first branch pushed to an empty repository"
  say ""
  if [ -z "$DRY" ] && [ "$(gh repo view "$EMPTY_REPO" --json isEmpty --jq .isEmpty)" != true ]; then
    say "skipped: $EMPTY_REPO is not empty"
  else
    EURL=https://github.com/$EMPTY_REPO.git
    remote git -C "$G1" push "$EURL" "refs/heads/$TRY:refs/heads/$TRY"
    if [ -z "$DRY" ]; then
      say "   default branch after pushing only \`$TRY\`: $(gh api "repos/$EMPTY_REPO" --jq .default_branch)"
    fi
    remote git -C "$G1" push "$EURL" "$MERGE:refs/heads/main"
    if [ -z "$DRY" ]; then
      say "   default branch after pushing main too: $(gh api "repos/$EMPTY_REPO" --jq .default_branch)"
    fi
  fi
  say ""
fi

say "Done. Results: $LOG"
