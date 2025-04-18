#!/bin/bash -e

echo -n "Version: "
read -r VERSION

echo -n "Message: "
read -r MESSAGE

git tag -s -a v"$VERSION" -m "$MESSAGE"
git push origin v"$VERSION"
