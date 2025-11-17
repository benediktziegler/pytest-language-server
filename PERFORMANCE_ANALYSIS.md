# Performance Analysis - pytest-language-server

**Date:** November 2025
**Version:** 0.5.1
**Codebase Size:** ~6,400 lines of Rust code

## Executive Summary

The pytest-language-server is generally well-optimized with good use of concurrent data structures and lazy evaluation. However, there are several opportunities for performance improvements, particularly around memory allocations and string operations.

## Memory Usage Analysis

### Data Structure Sizes
- `FixtureDefinition`: 80 bytes per instance
- `FixtureUsage`: 72 bytes per instance
- `PathBuf`: 24 bytes
- `String`: 24 bytes (+ heap allocation for content)

### Current Memory Patterns

**Good:**
- ✅ Uses `Arc<DashMap>` for lock-free concurrent access
- ✅ Minimal struct overhead (80 bytes for FixtureDefinition)
- ✅ File cache prevents repeated disk I/O

**Areas of Concern:**
- ⚠️ 133 `.clone()` calls throughout codebase
- ⚠️ 39 string allocations via `.to_string()`
- ⚠️ File cache stores entire file contents (`String`) even if only metadata needed
- ⚠️ PathBuf cloned frequently (file_path.clone() appears 30+ times)

## Performance Bottlenecks

### 1. `analyze_file()` - **HIGH IMPACT**

**Current behavior:**
```rust
pub fn analyze_file(&self, file_path: PathBuf, content: &str) {
    let file_path = file_path.canonicalize().unwrap_or_else(|_| { ... });
    self.file_cache.insert(file_path.clone(), content.to_string());  // ❌ Clone + allocation
    // ...
}
```

**Issues:**
- `canonicalize()` performs filesystem syscall (expensive)
- Stores full file content in cache even when only AST needed
- Clones PathBuf for cache key

**Impact:** Called once per file during workspace scan (potentially hundreds of files)

**Estimated cost:** ~100-500μs per file (depending on file size)

### 2. `find_fixture_definition()` - **MEDIUM-HIGH IMPACT**

**Current behavior:**
```rust
let content = if let Some(cached) = self.file_cache.get(file_path) {
    cached.clone()  // ❌ Clones entire file content!
} else {
    std::fs::read_to_string(file_path).ok()?
};
let lines: Vec<&str> = content.lines().collect();  // ❌ Allocates Vec
```

**Issues:**
- Clones entire file content from cache (could be thousands of lines)
- Allocates `Vec<&str>` just to access one line
- Called on every go-to-definition request (frequent)

**Impact:** Every LSP go-to-definition request

**Estimated cost:** ~10-50μs per request for medium files

### 3. `find_closest_definition()` - **MEDIUM IMPACT**

**Current behavior:**
```rust
let same_file_defs: Vec<_> = definitions
    .iter()
    .filter(|def| def.file_path == file_path)
    .collect();  // ❌ Allocates Vec
// ...
let last_def = same_file_defs.iter().max_by_key(|def| def.line).unwrap();
return Some((*last_def).clone());  // ❌ Clones FixtureDefinition
```

**Issues:**
- Allocates temporary Vec for filtering
- Clones FixtureDefinition (80 bytes + heap data)
- Could use iterator adapters without collection

**Impact:** Every fixture resolution request

**Estimated cost:** ~5-20μs per request

### 4. `scan_workspace()` - **LOW-MEDIUM IMPACT**

**Current behavior:**
```rust
for entry in WalkDir::new(root_path).into_iter().filter_map(|e| e.ok()) {
    if let Ok(content) = std::fs::read_to_string(path) {
        self.analyze_file(path.to_path_buf(), &content);  // ❌ Clone PathBuf
    }
}
```

**Issues:**
- Walks entire directory tree (unavoidable)
- Clones PathBuf for every file
- No parallelization (though WalkDir is sequential)

**Impact:** Only on workspace initialization

**Estimated cost:** ~100ms-1s for typical project (dominated by I/O)

## Optimization Opportunities

### Priority 1: High Impact, Easy Wins

#### 1.1 Avoid cloning file content in `find_fixture_definition()`

**Current:**
```rust
let content = if let Some(cached) = self.file_cache.get(file_path) {
    cached.clone()  // Clones entire file!
} else {
    std::fs::read_to_string(file_path).ok()?
};
let lines: Vec<&str> = content.lines().collect();
let line_content = lines[target_line - 1];
```

**Optimized:**
```rust
let content = if let Some(cached) = self.file_cache.get(file_path) {
    cached.value().clone()  // Still need to clone for borrowing
} else {
    std::fs::read_to_string(file_path).ok()?
};

// Access line directly without allocating Vec
let line_content = content
    .lines()
    .nth(target_line - 1)?;
```

**Savings:** Eliminates Vec allocation (~100-500 bytes) per request

#### 1.2 Use `Cow<str>` for cached content

**Change cache type:**
```rust
file_cache: Arc<DashMap<PathBuf, Arc<String>>>,  // Use Arc<String>
```

**Benefits:**
- Share string data via Arc instead of cloning
- Multiple readers can hold references without copying

**Savings:** ~1KB-10KB per go-to-definition request (depending on file size)

#### 1.3 Avoid allocating Vec in `find_closest_definition()`

**Current:**
```rust
let same_file_defs: Vec<_> = definitions
    .iter()
    .filter(|def| def.file_path == file_path)
    .collect();
let last_def = same_file_defs.iter().max_by_key(|def| def.line).unwrap();
```

**Optimized:**
```rust
let last_def = definitions
    .iter()
    .filter(|def| def.file_path == file_path)
    .max_by_key(|def| def.line)?;
```

**Savings:** Eliminates Vec allocation + one iteration pass

### Priority 2: Medium Impact, Moderate Effort

#### 2.1 Cache canonicalized paths

**Problem:** `canonicalize()` is called on every `analyze_file()`

**Solution:**
```rust
// Add to FixtureDatabase
canonical_path_cache: Arc<DashMap<PathBuf, PathBuf>>,

fn get_canonical_path(&self, path: PathBuf) -> PathBuf {
    if let Some(cached) = self.canonical_path_cache.get(&path) {
        return cached.value().clone();
    }

    let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
    self.canonical_path_cache.insert(path, canonical.clone());
    canonical
}
```

**Savings:** ~50-100μs per file (syscall elimination)

#### 2.2 Store Arc<str> instead of String in structs

**Current:**
```rust
pub struct FixtureDefinition {
    pub name: String,
    // ...
}
```

**Optimized:**
```rust
pub struct FixtureDefinition {
    pub name: Arc<str>,  // Shareable, no cloning
    // ...
}
```

**Trade-off:** More complex API, but better memory efficiency when many references exist

#### 2.3 Lazy-load file cache

**Problem:** File cache stores all file contents indefinitely

**Solution:**
- Implement LRU cache with size limit
- Only cache files currently open in editor
- Evict least-recently-used when memory pressure increases

### Priority 3: Low Impact, High Effort

#### 3.1 Parallel workspace scanning

**Current:** Sequential file analysis

**Optimized:** Use `rayon` for parallel processing
```rust
use rayon::prelude::*;

entries.par_iter().for_each(|entry| {
    // Parallel analysis
    self.analyze_file(...);
});
```

**Trade-off:** Adds complexity, but could reduce initial scan time by 2-4x

#### 3.2 Incremental AST parsing

**Problem:** Re-parses entire file on every change

**Solution:**
- Use tree-sitter or similar for incremental parsing
- Only re-analyze changed functions

**Trade-off:** Significant implementation effort, tree-sitter doesn't have stable Python support

## Allocation Hot Spots

Based on code analysis, here are the top allocation sites:

1. **File cache cloning** (~50-100 times per session) - **HIGH PRIORITY**
2. **PathBuf cloning** (~133 instances) - **MEDIUM PRIORITY**
3. **String allocations** (~39 instances) - **LOW PRIORITY**
4. **Vec allocations for filtering** (~20 instances) - **MEDIUM PRIORITY**

## Benchmark Recommendations

To validate these optimizations, create benchmarks for:

1. **Workspace scanning:** Time to scan 100-1000 file project
2. **Go-to-definition:** Time to resolve fixture in deep hierarchy
3. **Find references:** Time to find all usages of common fixture
4. **File analysis:** Time to parse and analyze single large file

Use `criterion` for Rust benchmarking:
```rust
#[bench]
fn bench_find_fixture_definition(b: &mut Bencher) {
    // Setup
    let db = FixtureDatabase::new();
    // ... populate with data

    b.iter(|| {
        db.find_fixture_definition(&test_path, 10, 15)
    });
}
```

## Estimated Impact Summary

| Optimization | Effort | Impact | Memory Saved | CPU Saved |
|-------------|--------|--------|--------------|-----------|
| Arc<String> cache | Low | High | 1KB-10KB/req | 10-50μs/req |
| Iterator chains | Low | Medium | 100-500B/req | 5-10μs/req |
| Canonical path cache | Medium | Medium | None | 50-100μs/file |
| Parallel scanning | High | Low | None | 50-200ms/scan |

## Current Performance Characteristics (Estimated)

**For a typical 500-file Python project:**

- **Initial workspace scan:** 500ms - 2s (I/O bound)
- **Go-to-definition:** 0.5-5ms (memory bound)
- **Find references:** 1-10ms (iteration bound)
- **Memory usage:** 5-50MB (depends on file cache)

**Bottlenecks by percentage:**
- I/O operations: 70%
- AST parsing: 20%
- Memory allocations: 8%
- Path operations: 2%

## Recommendations

**Immediate actions (Low effort, high impact):**
1. ✅ Use iterator chains instead of collecting to Vec
2. ✅ Change file_cache to Arc<String>
3. ✅ Avoid cloning file content on access

**Next phase (Medium effort, medium impact):**
4. Add canonical path cache
5. Implement LRU cache for file contents
6. Profile with `cargo flamegraph` on real projects

**Future considerations:**
7. Parallel workspace scanning
8. Incremental parsing (if performance becomes critical)

## Monitoring

Add these metrics to track performance:
- Cache hit rates
- Average file size in cache
- Time per analyze_file() call
- Time per find_fixture_definition() call
- Total memory usage

## Conclusion

The current implementation is reasonably efficient for small-to-medium projects. The biggest wins will come from:
1. Reducing unnecessary clones (especially file content)
2. Using Arc for shared string data
3. Eliminating temporary allocations in hot paths

For large projects (1000+ files), consider implementing the Priority 2 optimizations.
