# Bug Fix: Fixtures in Test Files Now Supported

## Problem
The LSP was only detecting fixtures defined in `conftest.py` files, but pytest allows fixtures to be defined in any test file.

When analyzing `/Users/bellini/dev/strawberry/tests/http/test_upload.py`, the logs showed:
```
DEBUG pytest_lsp::fixtures: Found function: http_client
DEBUG pytest_lsp::fixtures: Found function: enabled_http_client
```

But these functions had `@pytest.fixture` decorators and were never registered as fixtures!

## Root Cause
In `src/fixtures.rs`, line 111:
```rust
if is_conftest && is_fixture {
    // Register fixture definition
}
```

This condition required BOTH `is_conftest` AND `is_fixture` to be true, meaning fixtures in test files were ignored.

## Fix
Changed the condition to:
```rust
if is_fixture {
    // Register fixture definition
}
```

Now fixtures are detected regardless of whether they're in `conftest.py` or a test file.

## Testing
Added a new test `test_fixture_in_test_file()` that verifies:
- Fixtures defined in test files are detected
- Usages of those fixtures are detected
- Go-to-definition works for fixtures in the same file

All 5 tests now pass:
- ✅ test_fixture_definition_detection
- ✅ test_fixture_usage_detection
- ✅ test_go_to_definition
- ✅ test_fixture_decorator_variations
- ✅ test_fixture_in_test_file (NEW)

## What to Test
1. Clear the log file: `rm ~/.pytest_lsp.log`
2. Restart Neovim
3. Open `/Users/bellini/dev/strawberry/tests/http/test_upload.py`
4. Place cursor on `http_client` in line 35: `async def test_multipart_uploads_are_disabled_by_default(http_client: HttpClient):`
5. Press `gd` (go-to-definition)
6. Should jump to line 14 where `http_client` fixture is defined

## Expected Log Output
You should now see in `~/.pytest_lsp.log`:
```
INFO pytest_lsp::fixtures: Found fixture definition: http_client at ...
INFO pytest_lsp::fixtures: Found fixture usage: http_client at ...
INFO pytest_lsp: goto_definition request: ...
DEBUG pytest_lsp::fixtures: Found 1 usages in file
INFO pytest_lsp::fixtures: Found fixture name: http_client
INFO pytest_lsp::fixtures: Searching for definition of fixture: http_client
INFO pytest_lsp: Found definition: ...
```

The key difference: You should now see "Found fixture definition" messages for fixtures in test files!
