#!/bin/sh
set -eu

COMMIT_HASH=$(git rev-parse HEAD)
COMMIT_MESSAGE="Deploy $COMMIT_HASH"
ORIGIN="https://sagebind:$GITHUB_TOKEN@github.com/sagebind/riptide.git"

rm -rf target/docs/.git

git -C target/docs init .
git -C target/docs config user.name "Stephen M. Coakley"
git -C target/docs config user.email "me@stephencoakley.com"
git -C target/docs add -A
git -C target/docs commit -m "$COMMIT_MESSAGE"
git -C target/docs push "$ORIGIN" master:gh-pages --force

rm -rf target/docs/.git
