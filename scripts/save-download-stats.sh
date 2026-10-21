#!/usr/bin/env bash
# A separate, append-only data branch keeps counters out of reviewed application PRs.
set -euo pipefail
snapshot="site/public/stats/downloads.json"
ref="refs/heads/download-stats"
previous="$(git rev-parse --verify refs/remotes/origin/download-stats 2>/dev/null || true)"
if [[ -n "$previous" ]] && git show "$previous:downloads.json" | cmp -s - "$snapshot"; then
  exit 0
fi
stats_index="$(mktemp)"
rm "$stats_index"
trap 'rm -f "$stats_index"' EXIT
export GIT_INDEX_FILE="$stats_index"
git read-tree --empty
blob="$(git hash-object -w "$snapshot")"
git update-index --add --cacheinfo "100644,$blob,downloads.json"
tree="$(git write-tree)"
commit_stats() {
  git -c user.name='github-actions[bot]' -c user.email='41898282+github-actions[bot]@users.noreply.github.com' commit-tree "$tree" "$@" -m 'Refresh public release download counters'
}
if [[ -n "$previous" ]]; then
  commit="$(commit_stats -p "$previous")"
else
  commit="$(commit_stats)"
fi
git push origin "$commit:$ref"
