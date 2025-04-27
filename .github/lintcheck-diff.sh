#!/bin/bash

set -eu

body() {
    echo "Lintcheck changes for $(git rev-parse HEAD)"
    echo
    echo "$1"
    echo
    echo "This comment will be updated if you push new changes"
}

# --truncate so we don't hit the maximum size of 1MiB
# https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/workflow-commands-for-github-actions#step-isolation-and-limits
summary="$(./target/debug/lintcheck diff {base,head}/ci_crates_logs.json --truncate -o $GITHUB_STEP_SUMMARY)"

if [[ -n $summary ]]; then
    gh pr comment --body "$(body "$summary")" --edit-last --create-if-none "$@"
else
    # No changes - don't create a new comment but edit a previous one if it already exists
    gh pr comment --body "$(body "*No changes*")" --edit-last "$@" || true
fi
