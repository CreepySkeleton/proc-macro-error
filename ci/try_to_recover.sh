set -e

[[ "$(git branch --show-current)" != master ]] && exit 1

export TRYBUILD=overwrite
if cargo test --all ; then
    git config user.name "${GITLAB_USER_NAME}"
    git config user.email "${GITLAB_USER_EMAIL}"
    git add -A
    git commit -m "Adjust .stderr files"
    git push "http://${GITLAB_USER_NAME}:${GITLAB_PERSONAL_ACCESS_TOKEN}@gitlab.com/CreepySkeleton/proc-macro-error" HEAD:master
    exit 0
else
    exit 1
fi
