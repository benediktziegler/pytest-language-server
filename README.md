# pytest-lsp - Language Server for pytest fixtures

A Language Server Protocol (LSP) implementation in Rust that provides IDE support for pytest fixtures.

## Features

- **Fixture Detection**: Automatically finds fixture definitions in `conftest.py` files
- **Go-to-Definition**: Jump from fixture usage to definition
- **Workspace Scanning**: Discovers all test files and conftest files in your project
- **Real-time Updates**: Analyzes files as you edit them

## How It Works

1. **Fixture Definitions**: The LSP scans for functions decorated with `@pytest.fixture` or `@fixture` in:
   - `conftest.py` files
   - Test files (`test_*.py` and `*_test.py`)
2. **Fixture Usage**: Detects fixture usage by analyzing parameters in test functions (functions starting with `test_`)
3. **Go-to-Definition**: When you request go-to-definition on a fixture parameter, it finds the fixture definition in:
   - The same file (if defined there)
   - The closest `conftest.py` file in the directory hierarchy

## Usage with Neovim

Add this to your Neovim config:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.pytest_lsp then
  configs.pytest_lsp = {
    default_config = {
      cmd = {'/Users/bellini/dev/pytest-lsp/target/release/pytest-lsp'},
      filetypes = {'python'},
      root_dir = lspconfig.util.root_pattern('.git', 'pyproject.toml', 'setup.py'),
      settings = {},
    },
  }
end

lspconfig.pytest_lsp.setup{}
```

## Debugging / Logging

The LSP server logs detailed information to `~/.pytest_lsp.log`. You can monitor this in real-time:

```bash
tail -f ~/.pytest_lsp.log
```

The log includes:
- Workspace scanning progress
- Files being analyzed
- Fixtures found (definitions and usages)
- Go-to-definition requests and results
- Any errors or warnings

This is extremely helpful for debugging why go-to-definition might not be working.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/pytest-lsp`

## Testing

The project includes comprehensive unit tests:

```bash
cargo test
```

### Test Coverage

- ✅ Fixture definition detection (various decorator styles)
- ✅ Fixture usage detection (test function parameters)
- ✅ Go-to-definition functionality
- ✅ Multiple decorator variations (@pytest.fixture, @fixture, with/without parentheses)

## Example

Given these files:

**conftest.py**:
```python
import pytest

@pytest.fixture
def sample_fixture():
    return 42
```

**test_example.py**:
```python
def test_something(sample_fixture):
    assert sample_fixture == 42
```

When you place your cursor on `sample_fixture` in the test function and press `gd` (go-to-definition), it will jump to the fixture definition in `conftest.py`.

## Architecture

- **Parser**: Uses `rustpython-parser` for proper Python AST parsing
- **Storage**: Uses `DashMap` for thread-safe concurrent access to fixture definitions and usages
- **LSP Framework**: Built on `tower-lsp` for LSP protocol handling
- **File Discovery**: Uses `walkdir` for efficient workspace scanning

## Project Structure

```
src/
├── main.rs       # LSP server implementation
└── fixtures.rs   # Fixture detection and go-to-definition logic
```

## Current Limitations

- Does not support fixture scopes or complex fixture dependencies
- Does not provide hover information or completion yet
- Does not detect fixtures with custom decorators

## Future Enhancements

- Hover information showing fixture docstrings and return types
- Auto-completion for available fixtures in function parameters
- Show fixture scope (function, class, module, session)
- Fixture dependency graph visualization
- Support for parametrized fixtures
- Find all references to a fixture
