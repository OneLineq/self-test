#!/usr/bin/env bash
# ci/package-deb.sh — 打包 arm64 deb 包，从 tauri.conf.json 读取版本号
set -euo pipefail

# ── 读取配置 ──────────────────────────────────────────
VERSION=$(jq -r '.package.version' src-tauri/tauri.conf.json)
PKG_NAME="selftest"
ARCH="arm64"
PKG_DIR="${PKG_NAME}_${VERSION}_${ARCH}"
BINARY="SelfTest"
BINARY_PATH="src-tauri/target/aarch64-unknown-linux-gnu/release/${BINARY}"

echo "版本: ${VERSION}"
echo "包目录: ${PKG_DIR}"
echo "二进制: ${BINARY_PATH}"

# ── 校验二进制 ────────────────────────────────────────
if [ ! -f "${BINARY_PATH}" ]; then
  echo "❌ 二进制文件未找到: ${BINARY_PATH}"
  ls -la "src-tauri/target/aarch64-unknown-linux-gnu/release/"
  exit 1
fi
if [ ! -x "${BINARY_PATH}" ]; then
  echo "❌ 二进制文件不可执行: ${BINARY_PATH}"
  exit 1
fi

# ── 清理 ──────────────────────────────────────────────
rm -rf "${PKG_DIR}"

# ── 创建目录结构 ──────────────────────────────────────
mkdir -p "${PKG_DIR}/DEBIAN"
mkdir -p "${PKG_DIR}/usr/local/bin"
mkdir -p "${PKG_DIR}/usr/share/applications"
mkdir -p "${PKG_DIR}/usr/share/icons/hicolor/128x128/apps"
mkdir -p "${PKG_DIR}/usr/share/icons/hicolor/32x32/apps"
mkdir -p "${PKG_DIR}/usr/share/doc/${PKG_NAME}"

# ── 复制二进制 + 设置权限 ─────────────────────────────
install -Dm 755 "${BINARY_PATH}" "${PKG_DIR}/usr/local/bin/${PKG_NAME}"

# ── 复制图标 ──────────────────────────────────────────
if [ -f "src-tauri/icons/128x128.png" ]; then
  install -Dm 644 src-tauri/icons/128x128.png \
    "${PKG_DIR}/usr/share/icons/hicolor/128x128/apps/${PKG_NAME}.png"
fi
if [ -f "src-tauri/icons/32x32.png" ]; then
  install -Dm 644 src-tauri/icons/32x32.png \
    "${PKG_DIR}/usr/share/icons/hicolor/32x32/apps/${PKG_NAME}.png"
fi

# ── .desktop 文件 ────────────────────────────────────
printf '%s\n' \
  '[Desktop Entry]' \
  'Type=Application' \
  'Name=SelfTest' \
  'Name[zh_CN]=理论训练考核系统' \
  'GenericName=Quiz Practice Tool' \
  'Comment=理论训练考核系统 - Quiz practice app' \
  "Exec=env WEBKIT_DISABLE_COMPOSITING_MODE=1 ${PKG_NAME}" \
  "Icon=${PKG_NAME}" \
  'Terminal=false' \
  'Categories=Education;Utility;' \
  > "${PKG_DIR}/usr/share/applications/${PKG_NAME}.desktop"

# ── postinst ─────────────────────────────────────────
printf '%s\n' \
  '#!/bin/sh' \
  'set -e' \
  '' \
  'if command -v update-desktop-database >/dev/null 2>&1; then' \
  '    update-desktop-database /usr/share/applications 2>/dev/null || true' \
  'fi' \
  '' \
  'if command -v gtk-update-icon-cache >/dev/null 2>&1; then' \
  '    gtk-update-icon-cache /usr/share/icons/hicolor 2>/dev/null || true' \
  'fi' \
  '' \
  'if command -v update-mime-database >/dev/null 2>&1; then' \
  '    update-mime-database /usr/share/mime 2>/dev/null || true' \
  'fi' \
  '' \
  'exit 0' \
  > "${PKG_DIR}/DEBIAN/postinst"
chmod 755 "${PKG_DIR}/DEBIAN/postinst"

# ── prerm ────────────────────────────────────────────
printf '%s\n' \
  '#!/bin/sh' \
  'set -e' \
  '' \
  'exit 0' \
  > "${PKG_DIR}/DEBIAN/prerm"
chmod 755 "${PKG_DIR}/DEBIAN/prerm"

# ── postrm ───────────────────────────────────────────
printf '%s\n' \
  '#!/bin/sh' \
  'set -e' \
  '' \
  'if [ "$1" = "purge" ]; then' \
  '    if command -v update-desktop-database >/dev/null 2>&1; then' \
  '        update-desktop-database /usr/share/applications 2>/dev/null || true' \
  '    fi' \
  '    if command -v gtk-update-icon-cache >/dev/null 2>&1; then' \
  '        gtk-update-icon-cache /usr/share/icons/hicolor 2>/dev/null || true' \
  '    fi' \
  'fi' \
  '' \
  'exit 0' \
  > "${PKG_DIR}/DEBIAN/postrm"
chmod 755 "${PKG_DIR}/DEBIAN/postrm"

# ── control ──────────────────────────────────────────
printf '%s\n' \
  "Package: ${PKG_NAME}" \
  "Version: ${VERSION}" \
  "Architecture: ${ARCH}" \
  'Maintainer: oneline <oneline@example.com>' \
  'Depends: libwebkit2gtk-4.0-37, libgtk-3-0, libappindicator3-1' \
  'Section: education' \
  'Priority: optional' \
  'Description: SelfTest - Quiz practice app' \
  ' 理论训练考核系统是一款基于 Tauri 的刷题练习应用。' \
  ' 支持题库管理、练习模式、模拟考试等功能。' \
  > "${PKG_DIR}/DEBIAN/control"

# ── copyright ────────────────────────────────────────
printf '%s\n' \
  "Copyright $(date +%Y) oneline" \
  'License: MIT' \
  > "${PKG_DIR}/usr/share/doc/${PKG_NAME}/copyright"

# ── changelog ────────────────────────────────────────
{
  printf '%s (%s) unstable; urgency=medium\n\n' "${PKG_NAME}" "${VERSION}"
  printf '  * Initial release\n\n'
  printf ' -- oneline <oneline@example.com>  %s\n' "$(date -R)"
} > "${PKG_DIR}/usr/share/doc/${PKG_NAME}/changelog"
gzip -9 -n "${PKG_DIR}/usr/share/doc/${PKG_NAME}/changelog"

# ── 构建 ──────────────────────────────────────────────
echo "=== 构建 deb 包 ==="
dpkg-deb --build "${PKG_DIR}"

# ── 验证 ──────────────────────────────────────────────
echo "=== 包信息 ==="
dpkg-deb --info "${PKG_DIR}.deb"
echo "=== 包内容 ==="
dpkg-deb --contents "${PKG_DIR}.deb" | head -30

echo "✅ 打包完成: ${PKG_DIR}.deb"
