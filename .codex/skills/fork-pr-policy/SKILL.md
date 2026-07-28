---
name: fork-pr-policy
description: Enforce this workspace's Git branch and pull-request policy. Use whenever Codex creates or rebases a branch, pushes changes, opens or updates a pull request, selects a base branch or repository, or considers merging work in the nautilus_trader workspace.
---

# Apply The Repository Policy

Treat `arif-b-khan/nautilus_trader` as the writable repository and `main` as the base branch.

## Branches

- Create new work branches from the latest `origin/main`.
- Use the `codex/` prefix unless the user specifies another branch name.
- Do not base work on `develop` or an upstream branch.
- Before creating or rebasing a branch, verify that `origin` resolves to `arif-b-khan/nautilus_trader` and fetch `origin main`.
- Permit fetching from `upstream` only for read-only comparison or synchronization requested by the user. Never push to `upstream`.

## Pull Requests

- Set the pull request head to a branch in `arif-b-khan/nautilus_trader`.
- Set the pull request base repository to `arif-b-khan/nautilus_trader` and base branch to `main`.
- Verify the resolved owner, repository, and base branch immediately before creating or updating a pull request.
- Refuse to create or retarget a pull request whose base repository is the upstream NautilusTrader repository or whose base branch is `develop`.
- If a tool cannot explicitly guarantee the target repository and branch, stop and report the ambiguity instead of creating the pull request.

## Merges

- Never merge a pull request unless the user explicitly requests that specific merge.
- Treat approval to create, update, or publish a pull request as distinct from approval to merge it.
- Do not enable auto-merge unless the user explicitly requests it.

## Report

When publishing work, state the branch source and exact pull request target in the form `arif-b-khan/nautilus_trader:main`. Report that no merge was performed unless the user explicitly requested one.
