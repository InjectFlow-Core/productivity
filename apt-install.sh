#!/usr/bin/env bash
# Adds the daily-planner apt repository (hosted on GitHub Pages) and installs the package.
# Usage: curl -fsSL https://raw.githubusercontent.com/InjectFlow-Core/productivity/master/apt-install.sh | bash
set -e

REPO_URL="https://InjectFlow-Core.github.io/productivity"

echo "==> Adding daily-planner apt repository..."
curl -fsSL "${REPO_URL}/KEY.gpg" \
  | sudo gpg --dearmor -o /usr/share/keyrings/daily-planner.gpg

echo "deb [arch=amd64 signed-by=/usr/share/keyrings/daily-planner.gpg] ${REPO_URL} ./" \
  | sudo tee /etc/apt/sources.list.d/daily-planner.list > /dev/null

echo "==> Installing daily-planner..."
sudo apt-get update -q
sudo apt-get install -y daily-planner

echo ""
echo "Done! Run 'daily-planner --dashboard' to get started."
echo "Future updates: sudo apt upgrade"
