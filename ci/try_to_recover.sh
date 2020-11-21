set -e

apt-get install -y git

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
echo "BRANCH: $BRANCH"

[[ "$BRANCH" != master ]] && exit 1

echo "Trying to adjust .stderr files..."

export TRYBUILD=overwrite
if cargo test --all ; then
    echo "Adjustment succeeded"

    git config user.name "${GITLAB_USER_NAME}"
    git config user.email "${GITLAB_USER_EMAIL}"
    git add -A
    git commit -m "Adjust .stderr files"
    git push "http://${GITLAB_USER_NAME}:${GITLAB_PERSONAL_ACCESS_TOKEN}@gitlab.com/CreepySkeleton/proc-macro-error" HEAD:master
    exit 0
else
    echo "Adjustment failed"
    exit 1
fi
