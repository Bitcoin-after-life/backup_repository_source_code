# BAL source-code backup

Automatic, weekly off-site backup of **all public repositories** of the
[Bitcoin After Life Gitea organisation](https://bitcoin-after.life/gitea/bitcoinafterlife)
into this GitHub repository.

## What is stored

Every run rebuilds the tree under `repos/`. For each source repository you get:

```
repos/<name>/
├── <name>.bundle   # full git bundle: complete history, every branch + tag
└── source/         # checked-out working tree of the default branch (browsable here)
```

- **`<name>.bundle`** is the authoritative, fully restorable backup. It contains
  the entire git history (all branches and tags), packed into a single file.
- **`source/`** is the plain working tree of the default branch, so the code is
  readable directly here on GitHub without restoring anything.

A top-level [`MANIFEST.md`](./MANIFEST.md) lists every backed-up repository with
its default branch, last commit and timestamp of the run.

## Schedule

The backup runs automatically via GitHub Actions:

- **Every Sunday at 03:17 UTC** (`schedule` cron in
  [`.github/workflows/backup-from-gitea.yml`](./.github/workflows/backup-from-gitea.yml)).
- It can also be triggered manually from the **Actions** tab
  (**Run workflow** → *Weekly Gitea backup*).

The workflow is **idempotent**: it re-mirrors from Gitea each time, so deletions,
force-pushes and renamed branches on the source are reflected in the backup.

## Failure alerts

If a scheduled (or manual) backup fails, the workflow automatically opens a
GitHub **issue** labelled `backup-failure` with a link to the failed run, so the
failure is never silent. If an alert is already open, a comment is added instead
of creating a duplicate. On the next **successful** run the open alert is
commented and **closed automatically**.

## No personal token required

The workflow writes its commits using GitHub's built-in `GITHUB_TOKEN`, **not**
a personal access token. The Gitea source repositories are public, so no
credentials are needed to read them either.

> A personal access token was only used **once**, for the initial setup
> (creating this README, the workflow and the script). That token can be safely
> revoked — the weekly backup keeps running indefinitely.

## How to restore a repository

Download the bundle for the repo you want, then clone from it:

```bash
# 1. Get the file repos/<name>/<name>.bundle from this repository.
# 2. Restore the full repository (all branches + tags) from it:
git clone <name>.bundle <name>
cd <name>
git remote -v          # the bundle is set as 'origin'; repoint it if needed
git branch -a          # all branches are present
git tag                # all tags are present
```

To restore just the latest source code without git history, copy the contents
of `repos/<name>/source/`.

## Files

| Path | Purpose |
|---|---|
| `.github/workflows/backup-from-gitea.yml` | Scheduled GitHub Actions workflow |
| `scripts/backup_from_gitea.sh` | Backup logic (lists Gitea repos, builds bundles + source) |
| `repos/` | The backup tree (generated) |
| `MANIFEST.md` | Per-repo summary of the last run (generated) |
