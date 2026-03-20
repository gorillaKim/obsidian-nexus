#!/bin/bash
# Usage: ./scripts/bump-version.sh 0.4.0
# Bumps version in all config files, commits, tags, and pushes

set -e

VERSION=$1

if [ -z "$VERSION" ]; then
  # Auto-increment patch version from Cargo.toml
  CURRENT=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
  MAJOR=$(echo "$CURRENT" | cut -d. -f1)
  MINOR=$(echo "$CURRENT" | cut -d. -f2)
  PATCH=$(echo "$CURRENT" | cut -d. -f3)
  VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))"
  echo "Auto-incrementing: ${CURRENT} → ${VERSION}"
fi

echo "Bumping to v${VERSION}..."

# Update all version files
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
sed -i '' "s/\"version\": \".*\"/\"version\": \"${VERSION}\"/" apps/desktop/src-tauri/tauri.conf.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"${VERSION}\"/" apps/desktop/package.json

echo "Updated:"
echo "  Cargo.toml            → ${VERSION}"
echo "  tauri.conf.json       → ${VERSION}"
echo "  package.json          → ${VERSION}"

# Commit, tag, push
git add Cargo.toml apps/desktop/src-tauri/tauri.conf.json apps/desktop/package.json
git commit -m "chore: bump version to ${VERSION}"
git tag "v${VERSION}"
git push && git push origin "v${VERSION}"

echo ""
echo "✓ v${VERSION} released! CI will build and upload artifacts."
