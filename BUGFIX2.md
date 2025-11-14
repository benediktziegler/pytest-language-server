# Bug Fix #2: Async Test Functions Now Supported

## Problem
The LSP was detecting fixture definitions but NOT detecting fixture usages. From the logs:

```
INFO Found fixture definition: http_client at line 14
INFO Found fixture definition: enabled_http_client at line 25
...
INFO No fixture definition found  <-- No usages detected!
```

The test function was:
```python
async def test_multipart_uploads_are_disabled_by_default(http_client: HttpClient):
```

## Root Cause
The code only handled `Stmt::FunctionDef` but pytest supports **async test functions** which use `Stmt::AsyncFunctionDef` in the AST.

## Fix
Refactored `visit_stmt()` to handle both:
- `Stmt::FunctionDef` - regular functions
- `Stmt::AsyncFunctionDef` - async functions

Both now extract:
- Function name
- Decorators (for fixture detection)
- Arguments (for fixture usage detection)

## Testing
Added `test_async_test_functions()` which verifies:
- Async test functions are detected
- Sync test functions are still detected
- Fixture usages in both types are found

**All 6 tests pass** ✅

## What to Do Now

1. **Restart Neovim completely** (kill all nvim processes to ensure new binary is used)

2. **Clear logs and watch**:
   ```bash
   rm ~/.pytest_lsp.log
   tail -f ~/.pytest_lsp.log
   ```

3. **Test it**:
   - Open `/Users/bellini/dev/strawberry/tests/http/test_upload.py`
   - Line 35: `async def test_multipart_uploads_are_disabled_by_default(http_client: HttpClient):`
   - Put cursor on `http_client` (the parameter)
   - Press `gd`
   - **Should jump to line 14!**

4. **Check logs** - you should now see:
   ```
   INFO Found fixture definition: http_client at ...line 14
   INFO Found fixture usage: http_client at ...line 35
   DEBUG Found 1 usages in file
   INFO Found fixture name: http_client
   INFO Found definition: ...
   ```

The key difference: You'll now see **"Found fixture usage"** messages!

## Files Changed
- `src/fixtures.rs`: Updated `visit_stmt()` to handle async functions
- Added comprehensive test coverage

Binary rebuilt at: `/Users/bellini/dev/pytest-lsp/target/release/pytest-lsp`
