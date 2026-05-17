#!/usr/bin/env bash
set -e

BINARY_DEST="$HOME/.local/bin/daily-planner"
SYSTEMD_DIR="$HOME/.config/systemd/user"
DESKTOP_FILE="$HOME/.local/share/applications/daily-planner.desktop"
ICON_DEST="$HOME/.local/share/icons/daily-planner.png"

UNITS=(
  daily-planner-login.service
  daily-planner.timer
  daily-planner-timer.service
  daily-planner-evening.timer
  daily-planner-evening-trigger.service
)

echo "==> Stopping and disabling systemd units"
for unit in "${UNITS[@]}"; do
  systemctl --user stop "$unit" 2>/dev/null || true
  systemctl --user disable "$unit" 2>/dev/null || true
  rm -f "$SYSTEMD_DIR/$unit"
done
systemctl --user daemon-reload

echo "==> Removing binary"
rm -f "$BINARY_DEST"

echo "==> Removing .desktop entry and icon"
rm -f "$DESKTOP_FILE"
rm -f "$ICON_DEST"
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true

echo ""
echo "Daily Planner uninstalled."
