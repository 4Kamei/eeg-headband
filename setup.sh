#!/usr/bin/env bash

git remote remove origin
git checkout --orphan temp_branch

sed -i "s/PROJECT_NAME/$(basename $(pwd))/" Cargo.toml

git add Cargo.toml
git add .envrc
git add .gitignore
git add flake.nix
git commit -m "Init"

git branch -D master
git branch -m master

rm setup.sh

