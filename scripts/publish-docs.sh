#!/bin/sh
set -eu

COMMIT_HASH=$(git rev-parse HEAD)
COMMIT_MESSAGE="Deploy $COMMIT_HASH"
ORIGIN="https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git"

rm -rf target/docs/.git
touch target/docs/.nojekyll

git -C target/docs init .
git -C target/docs config user.name "${GITHUB_ACTOR}"
git -C target/docs config user.email "${GITHUB_ACTOR}@users.noreply.github.com"
git -C target/docs add .
git -C target/docs commit -m "${COMMIT_MESSAGE}"
git -C target/docs push "${ORIGIN}" master:gh-pages --force

rm -rf target/docs/.git
