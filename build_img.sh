#!/usr/bin/env bash
set -euo pipefail

# Get current git branch name
BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Normalize branch name (replace / with - for Docker tag safety)
TAG=${BRANCH//\//-}

echo "Building Docker image: bamstorm:${TAG}"

docker build -t bamstorm:${TAG} .