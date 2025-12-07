# Editor Extensions

This directory contains editor/IDE extensions for pytest-language-server:

| Extension | Directory | Marketplace |
|-----------|-----------|-------------|
| VS Code | [`vscode-extension/`](./vscode-extension/) | [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=bellini666.pytest-language-server) |
| IntelliJ/PyCharm | [`intellij-plugin/`](./intellij-plugin/) | [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/26096-pytest-language-server) |
| Zed | [`zed-extension/`](./zed-extension/) | [Zed Extensions](https://zed.dev/extensions?query=pytest) |

## For Users

Install extensions directly from your editor's extension marketplace. Each extension bundles the language server binary - no additional installation required.

See individual extension READMEs for configuration options.

## For Developers

### Local Development

**VS Code:**
```bash
cd vscode-extension
npm install
# Press F5 in VS Code to launch Extension Development Host
```

**IntelliJ:**
```bash
cd intellij-plugin
# Install pytest-language-server in PATH first:
cargo install --path ../..
# Then launch IDE with plugin:
./gradlew runIde -Dpytest.lsp.useSystemPath=true
```

**Zed:**
```bash
cd zed-extension
rustup target add wasm32-wasip1
# In Zed: "zed: install dev extension" → select this directory
```

### Release Process

Extensions are published automatically when a GitHub release is created:

1. Bump version: `./bump-version.sh X.Y.Z`
2. Commit: `git commit -am "chore: bump version to X.Y.Z"`
3. Tag and push: `git tag vX.Y.Z && git push && git push --tags`

CI automatically:
- Builds platform-specific binaries
- Packages each extension with bundled binaries
- Publishes to VS Code Marketplace (`VSCE_TOKEN`)
- Publishes to JetBrains Marketplace (`JETBRAINS_TOKEN`)
- Creates GitHub release with Zed extension

### Required GitHub Secrets

| Secret | Purpose |
|--------|---------|
| `VSCE_TOKEN` | VS Code Marketplace publishing |
| `JETBRAINS_TOKEN` | JetBrains Marketplace publishing |
| `CARGO_REGISTRY_TOKEN` | crates.io publishing |

### Binary Resolution

All extensions follow this priority:
1. User-configured path (if set)
2. System PATH (if `pytest-language-server` is installed)
3. Bundled binary (platform-specific)

### Version Synchronization

The `bump-version.sh` script updates versions in:
- `Cargo.toml` (main project)
- `pyproject.toml`
- `zed-extension/Cargo.toml`
- `zed-extension/extension.toml`

**Manually updated by CI:**
- `vscode-extension/package.json`
- `intellij-plugin/build.gradle.kts`
