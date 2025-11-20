# TODO for Zed Extension

## Design Philosophy

This extension follows a **hybrid approach**:
1. **Priority 1**: Check PATH first (user installs via pip, uv, cargo, homebrew)
2. **Priority 2**: Auto-download from GitHub releases with version management

This provides both simplicity for power users and convenience for new users.

## Future Improvements

### Nice-to-Have Features

1. **Better Error Messages**
   - Detect if user is in a virtual environment without pytest-language-server
   - Suggest installation command based on detected package manager
   - Link to installation docs

2. **Configuration Options**
   - Allow users to specify custom command-line arguments
   - Support for `RUST_LOG` environment variable configuration
   - Workspace-specific settings

3. **Status Indicators**
   - Show when the language server is starting/ready
   - Display version information
   - Health check diagnostics

## Contributing

If you want to add features, please:
1. Keep it simple
2. Test with different installation methods (pip, cargo, brew)
3. Test in virtual environments
4. Consider cross-platform compatibility
