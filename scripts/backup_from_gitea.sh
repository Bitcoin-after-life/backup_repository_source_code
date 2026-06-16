#!/usr/bin/env bash
#
# Weekly backup of all public repositories of a Gitea organisation into this
# GitHub repository.
#
# For every source repo we store TWO things under repos/<name>/ :
#   1. <name>.bundle  -- a full `git bundle` (complete history: every branch +
#                        tag).  This is the authoritative, restorable backup.
#                        Restore with:  git clone <name>.bundle <name>
#   2. source/        -- the checked-out working tree of the default branch,
#                        so the code is browsable directly on GitHub.
#
# The script is idempotent: it rebuilds the mirror from scratch each run, so
# deletions / force-pushes / renamed branches on Gitea are reflected too.
#
# Env vars:
#   GITEA_BASE   e.g. https://bitcoin-after.life/gitea
#   GITEA_ORG    e.g. bitcoinafterlife
#
set -euo pipefail

GITEA_BASE="${GITEA_BASE:?GITEA_BASE not set}"
GITEA_ORG="${GITEA_ORG:?GITEA_ORG not set}"
API="${GITEA_BASE%/}/api/v1"

WORKDIR="$(pwd)"
REPOS_DIR="${WORKDIR}/repos"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "==> Gitea base : $GITEA_BASE"
echo "==> Gitea org  : $GITEA_ORG"

# --- 1. Fetch the list of repositories from the Gitea API (paginated) --------
echo "==> Listing repositories of '$GITEA_ORG' ..."
# Detect once which collection endpoint exists for this owner. In Gitea an
# owner can be an organisation (/orgs/<name>) or a plain user (/users/<name>);
# the wrong one returns 404. Picking it once avoids retrying a 404 on every page.
endpoint=""
for kind in orgs users; do
  if curl -fsSL -o /dev/null -H 'Accept: application/json' \
      "${API}/${kind}/${GITEA_ORG}/repos?limit=1&page=1" 2>/dev/null; then
    endpoint="$kind"
    break
  fi
done
if [ -z "$endpoint" ]; then
  echo "ERROR: neither /orgs/${GITEA_ORG} nor /users/${GITEA_ORG} is reachable." >&2
  exit 1
fi
echo "==> Using Gitea endpoint: /${endpoint}/${GITEA_ORG}"

page=1
clone_urls=()
names=()
while : ; do
  resp="$(curl -fsSL -H 'Accept: application/json' \
    "${API}/${endpoint}/${GITEA_ORG}/repos?limit=50&page=${page}")"

  count="$(printf '%s' "$resp" | python3 -c 'import sys,json; print(len(json.load(sys.stdin)))')"
  [ "$count" -eq 0 ] && break

  while IFS=$'\t' read -r name clone; do
    [ -z "$name" ] && continue
    names+=("$name")
    clone_urls+=("$clone")
  done < <(printf '%s' "$resp" | python3 -c '
import sys, json
for r in json.load(sys.stdin):
    print("%s\t%s" % (r["name"], r["clone_url"]))
')
  page=$((page + 1))
done

total="${#names[@]}"
echo "==> Found ${total} repositories."
if [ "$total" -eq 0 ]; then
  echo "ERROR: no repositories returned by the Gitea API." >&2
  exit 1
fi

# --- 2. Mirror each repository ------------------------------------------------
rm -rf "$REPOS_DIR"
mkdir -p "$REPOS_DIR"

manifest="${WORKDIR}/MANIFEST.md"
{
  echo "# Backup manifest"
  echo
  echo "Source: \`${GITEA_BASE}/${GITEA_ORG}\`"
  echo
  echo "Last run (UTC): \`$(date -u '+%Y-%m-%d %H:%M:%S')\`"
  echo
  echo "| Repository | Default branch | Last commit | Date |"
  echo "|---|---|---|---|"
} > "$manifest"

for i in "${!names[@]}"; do
  name="${names[$i]}"
  url="${clone_urls[$i]}"
  echo
  echo "==> [$((i+1))/${total}] ${name}"

  mirror="${TMP_DIR}/${name}.git"
  # Bare mirror clone = all refs (branches + tags), full history.
  git clone --quiet --mirror "$url" "$mirror"

  dest="${REPOS_DIR}/${name}"
  mkdir -p "$dest"

  # 2a. Full bundle (authoritative restorable backup).
  ( cd "$mirror" && git bundle create "${dest}/${name}.bundle" --all >/dev/null 2>&1 )

  # 2b. Browsable working tree of the default branch.
  defbranch="$(cd "$mirror" && git symbolic-ref --short HEAD 2>/dev/null || echo main)"
  worktree="${dest}/source"
  rm -rf "$worktree"
  git clone --quiet "$mirror" "$worktree" >/dev/null 2>&1 || true
  # Drop the inner .git so this repo does not become a nested git repo / submodule.
  rm -rf "${worktree}/.git"

  last_commit="$(cd "$mirror" && git log -1 --format='%h %s' 2>/dev/null | cut -c1-60 || echo '-')"
  last_date="$(cd "$mirror" && git log -1 --format='%ci' 2>/dev/null || echo '-')"
  printf '| `%s` | `%s` | %s | %s |\n' \
    "$name" "$defbranch" "${last_commit//|/\\|}" "$last_date" >> "$manifest"

  echo "    bundle: $(du -h "${dest}/${name}.bundle" | cut -f1)  | default branch: ${defbranch}"
done

echo
echo "==> Backup tree built under: ${REPOS_DIR}"
echo "==> Manifest written to:     ${manifest}"
