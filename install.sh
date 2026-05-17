#!/usr/bin/env bash
set -e

BINARY_SRC="src-tauri/target/release/daily-planner"
BINARY_DEST="$HOME/.local/bin/daily-planner"
SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "==> Checking build..."
if [ ! -f "$BINARY_SRC" ]; then
  echo "Binary not found. Building..."
  npm install
  npm run build
fi

echo "==> Installing binary to $BINARY_DEST"
mkdir -p "$HOME/.local/bin"
cp "$BINARY_SRC" "$BINARY_DEST"
chmod +x "$BINARY_DEST"

echo "==> Installing .desktop file"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_SRC="src-tauri/icons/icon.png"
ICON_DEST="$HOME/.local/share/icons/daily-planner.png"
mkdir -p "$DESKTOP_DIR" "$(dirname "$ICON_DEST")"
[ -f "$ICON_SRC" ] && cp "$ICON_SRC" "$ICON_DEST"
cat > "$DESKTOP_DIR/daily-planner.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Daily Planner
Comment=Plan your day and review your progress
Exec=$BINARY_DEST --dashboard
Icon=$ICON_DEST
Terminal=false
Categories=Office;ProjectManagement;
StartupWMClass=daily-planner
EOF
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo "==> Installing systemd units"
mkdir -p "$SYSTEMD_DIR"
cp systemd/daily-planner-login.service        "$SYSTEMD_DIR/"
cp systemd/daily-planner.timer                "$SYSTEMD_DIR/"
cp systemd/daily-planner-timer.service        "$SYSTEMD_DIR/"
cp systemd/daily-planner-evening.timer        "$SYSTEMD_DIR/"
cp systemd/daily-planner-evening-trigger.service "$SYSTEMD_DIR/"

echo "==> Enabling services"
systemctl --user daemon-reload
systemctl --user enable --now daily-planner-login.service
systemctl --user enable --now daily-planner.timer
systemctl --user enable --now daily-planner-evening.timer

echo ""
echo "Done!"
echo ""
echo "Configured times (edit these files to change):"
echo "  Morning lock  : $SYSTEMD_DIR/daily-planner.timer         (OnCalendar=*-*-* 05:00:00)"
echo "  Evening review: $SYSTEMD_DIR/daily-planner-evening.timer  (OnCalendar=*-*-* 18:00:00)"
echo ""
echo "After editing a timer, reload with:"
echo "  systemctl --user daemon-reload && systemctl --user restart <timer-name>"
echo ""
echo "Open the dashboard anytime:"
echo "  daily-planner --dashboard"
