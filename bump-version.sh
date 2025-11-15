#!/bin/bash
# Version bump script for pytest-language-server
# Usage: ./bump-version.sh <new-version>
# Example: ./bump-version.sh 0.3.1

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.3.1"
    exit 1
fi

NEW_VERSION="$1"

# Validate version format (basic semver check)
if ! echo "$NEW_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "Error: Version must be in format X.Y.Z (e.g., 0.3.1)"
    exit 1
fi

echo "Bumping version to $NEW_VERSION..."

# Update Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml && rm Cargo.toml.bak

# Update pyproject.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" pyproject.toml && rm pyproject.toml.bak

# Update zed-extension/Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" zed-extension/Cargo.toml && rm zed-extension/Cargo.toml.bak

# Update zed-extension/extension.toml
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" zed-extension/extension.toml && rm zed-extension/extension.toml.bak

# Update Cargo.lock
cargo update -p pytest-language-server

echo "✓ Version bumped to $NEW_VERSION in:"
echo "  - Cargo.toml"
echo "  - pyproject.toml"
echo "  - zed-extension/Cargo.toml"
echo "  - zed-extension/extension.toml"
echo "  - Cargo.lock"
echo ""
echo "Run 'git add -A && git commit -m \"chore: bump version to $NEW_VERSION\"' to commit"
