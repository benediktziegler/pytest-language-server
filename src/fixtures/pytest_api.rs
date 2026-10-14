//! Mapping from pytest's private `_pytest.*` names to the public `pytest` API.
//!
//! Fixture return annotations frequently reference types that live in
//! `_pytest` (e.g. `MonkeyPatch` from `_pytest.monkeypatch`, or
//! `CaptureFixture` from `_pytest.capture`).  Inserting an import from a
//! private module into user code is undesirable — pytest re-exports all of
//! these from the top-level `pytest` package.
//!
//! # Design
//!
//! * [`PytestPublicApi`] maps `"<private module>.<name>"` to the public
//!   `pytest.<name>`.  It is learned from the installed pytest's own
//!   `pytest/__init__.py` ([`PytestPublicApi::from_init_source`]).  When pytest
//!   is not found in site-packages nothing is learned and the original
//!   `_pytest` imports are left untouched.
//! * [`publicize_pytest_types`] is a **pure** rewrite applied at consumption
//!   time (code actions, inlay hints) — *not* when fixtures are analysed.
//!   Workspace files are analysed before the venv is scanned, so the mapping
//!   is not available yet at analysis time.  Stored `FixtureDefinition`s are
//!   therefore left untouched.
//! * [`FixtureDatabase::adapt_return_type_for_consumer`] composes the two
//!   steps: publicise first, then adapt to the consumer's import style, so
//!   `import pytest` in the consumer yields `pytest.MonkeyPatch` with no new
//!   import.

use super::import_analysis::{adapt_type_for_consumer, split_from_import};
use super::types::TypeImportSpec;
use super::FixtureDatabase;
use rustpython_parser::ast::{Constant, Expr, Mod, Stmt};
use rustpython_parser::Mode;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Top-level name of pytest's private package.
const PRIVATE_ROOT: &str = "_pytest";

/// Lookup table from private `_pytest` definitions to their public name in
/// the `pytest` namespace.
#[derive(Debug, Default, Clone)]
pub struct PytestPublicApi {
    /// `"_pytest.capture.CaptureFixture"` → `"CaptureFixture"`.
    by_private: HashMap<String, String>,
}

impl PytestPublicApi {
    /// Build the table from the source of `pytest/__init__.py`.
    ///
    /// Only `from _pytest[.x] import Name [as Public]` statements are
    /// considered.  When the file defines `__all__`, only names listed in it
    /// are kept; underscore-prefixed names are always dropped.
    ///
    /// Returns `None` if the source does not parse or yields no entries.
    pub fn from_init_source(source: &str) -> Option<Self> {
        let Mod::Module(module) =
            rustpython_parser::parse(source, Mode::Module, "pytest/__init__.py").ok()?
        else {
            return None;
        };

        let mut all_names: Option<HashSet<String>> = None;
        // (private module, original name, public name)
        let mut candidates: Vec<(String, String, String)> = Vec::new();

        for stmt in &module.body {
            match stmt {
                Stmt::ImportFrom(import_from) => {
                    if import_from.level.as_ref().is_some_and(|l| l.to_usize() > 0) {
                        continue;
                    }
                    let Some(module_name) = import_from.module.as_ref() else {
                        continue;
                    };
                    if !is_private_module(module_name.as_str()) {
                        continue;
                    }
                    for alias in &import_from.names {
                        if alias.name.as_str() == "*" {
                            continue;
                        }
                        let public = alias.asname.as_ref().unwrap_or(&alias.name);
                        candidates.push((
                            module_name.to_string(),
                            alias.name.to_string(),
                            public.to_string(),
                        ));
                    }
                }
                Stmt::Assign(assign) => {
                    if assign.targets.iter().any(is_all_target) {
                        all_names = Some(string_elements(&assign.value));
                    }
                }
                Stmt::AnnAssign(ann_assign) if is_all_target(&ann_assign.target) => {
                    if let Some(value) = ann_assign.value.as_deref() {
                        all_names = Some(string_elements(value));
                    }
                }
                _ => {}
            }
        }

        let by_private: HashMap<String, String> = candidates
            .into_iter()
            .filter(|(_, _, public)| {
                !public.starts_with('_')
                    && all_names.as_ref().is_none_or(|all| all.contains(public))
            })
            .map(|(module, original, public)| (format!("{module}.{original}"), public))
            .collect();

        if by_private.is_empty() {
            None
        } else {
            Some(Self { by_private })
        }
    }

    /// Public `pytest` name for `name` defined in the private `module`.
    pub fn public_name(&self, module: &str, name: &str) -> Option<&str> {
        self.by_private
            .get(&format!("{module}.{name}"))
            .map(String::as_str)
    }

    /// Number of known mappings.
    pub(crate) fn len(&self) -> usize {
        self.by_private.len()
    }
}

fn is_all_target(expr: &Expr) -> bool {
    matches!(expr, Expr::Name(name) if name.id.as_str() == "__all__")
}

fn string_elements(expr: &Expr) -> HashSet<String> {
    let elts = match expr {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        _ => return HashSet::new(),
    };
    elts.iter()
        .filter_map(|elt| match elt {
            Expr::Constant(c) => match &c.value {
                Constant::Str(s) => Some(s.to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// `true` for `_pytest` and any `_pytest.<sub>` module — but not for
/// look-alikes such as `_pytest_foo`.
fn is_private_module(module: &str) -> bool {
    module == PRIVATE_ROOT
        || module
            .strip_prefix(PRIVATE_ROOT)
            .is_some_and(|rest| rest.starts_with('.'))
}

// ─── publicize_pytest_types ───────────────────────────────────────────────────

/// Rewrite a fixture's return type and import specs so they reference the
/// public `pytest` API instead of private `_pytest` modules.
///
/// * `from _pytest.capture import CaptureFixture` →
///   `from pytest import CaptureFixture` (aliases are preserved, so the type
///   string needs no change).
/// * Dotted references such as `_pytest.capture.CaptureFixture[str]` (via
///   `import _pytest.capture`) or `cap.CaptureFixture[str]` (via
///   `import _pytest.capture as cap`) are rewritten to
///   `pytest.CaptureFixture[str]`, and the now-unused private bare import is
///   replaced by `import pytest`.
///
/// Names without a public equivalent (e.g. `SubRequest`) and everything
/// outside `_pytest` are left untouched.
///
/// Returns `(rewritten_type, rewritten_specs)`.
pub fn publicize_pytest_types(
    return_type: &str,
    specs: &[TypeImportSpec],
    api: &PytestPublicApi,
) -> (String, Vec<TypeImportSpec>) {
    if !specs
        .iter()
        .any(|s| s.import_statement.contains(PRIVATE_ROOT))
    {
        return (return_type.to_string(), specs.to_vec());
    }

    // `import _pytest.x as alias` → alias → "_pytest.x"
    let mut aliases: HashMap<&str, &str> = HashMap::new();
    for spec in specs {
        let Some(rest) = spec.import_statement.strip_prefix("import ") else {
            continue;
        };
        if let Some((module, _alias)) = rest.split_once(" as ") {
            let module = module.trim();
            if is_private_module(module) {
                aliases.insert(spec.check_name.as_str(), module);
            }
        }
    }

    let (rewritten, changed) = rewrite_dotted_refs(return_type, &aliases, api);

    let mut out: Vec<TypeImportSpec> = Vec::new();
    let mut push = |spec: TypeImportSpec| {
        if !out.contains(&spec) {
            out.push(spec);
        }
    };

    for spec in specs {
        if let Some(public_spec) = publicize_from_import(spec, api) {
            push(public_spec);
            continue;
        }
        if changed && is_private_bare_import(spec) && !references_root(&rewritten, &spec.check_name)
        {
            // Every dotted use of this import was rewritten to `pytest.X`.
            continue;
        }
        push(spec.clone());
    }

    if changed {
        push(TypeImportSpec {
            check_name: "pytest".to_string(),
            import_statement: "import pytest".to_string(),
        });
    }

    (rewritten, out)
}

/// Rewrite `from _pytest.x import Name [as Alias]` to the public form.
fn publicize_from_import(spec: &TypeImportSpec, api: &PytestPublicApi) -> Option<TypeImportSpec> {
    let (module, name_part) = split_from_import(&spec.import_statement)?;
    if !is_private_module(module) {
        return None;
    }
    let original = name_part.split(" as ").next().unwrap_or(name_part).trim();
    let public = api.public_name(module, original)?;
    let import_statement = if public == spec.check_name {
        format!("from pytest import {public}")
    } else {
        format!("from pytest import {public} as {}", spec.check_name)
    };
    Some(TypeImportSpec {
        check_name: spec.check_name.clone(),
        import_statement,
    })
}

/// `import _pytest.x` or `import _pytest.x as y`.
fn is_private_bare_import(spec: &TypeImportSpec) -> bool {
    spec.import_statement
        .strip_prefix("import ")
        .map(|rest| rest.split(" as ").next().unwrap_or("").trim())
        .is_some_and(is_private_module)
}

/// Ranges of maximal dotted identifier runs (`a.b.c`) in `text`.
fn dotted_tokens(text: &str) -> Vec<(usize, usize)> {
    fn ident_end(bytes: &[u8], mut i: usize) -> usize {
        i += 1;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        i
    }
    fn ident_start(b: u8) -> bool {
        b.is_ascii_alphabetic() || b == b'_'
    }

    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if ident_start(bytes[i]) {
            let start = i;
            i = ident_end(bytes, i);
            while i + 1 < bytes.len() && bytes[i] == b'.' && ident_start(bytes[i + 1]) {
                i = ident_end(bytes, i + 1);
            }
            tokens.push((start, i));
        } else {
            i += 1;
        }
    }
    tokens
}

/// `true` if any dotted token in `text` begins with the segment `root`.
fn references_root(text: &str, root: &str) -> bool {
    dotted_tokens(text)
        .into_iter()
        .any(|(s, e)| text[s..e].split('.').next() == Some(root))
}

/// Replace `<private root>.<sub…>.<Name>` tokens with `pytest.<Public>`.
/// Returns the new string and whether anything changed.
fn rewrite_dotted_refs(
    text: &str,
    aliases: &HashMap<&str, &str>,
    api: &PytestPublicApi,
) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut changed = false;

    for (start, end) in dotted_tokens(text) {
        let Some(replacement) = rewrite_token(&text[start..end], aliases, api) else {
            continue;
        };
        out.push_str(&text[last..start]);
        out.push_str(&replacement);
        last = end;
        changed = true;
    }
    out.push_str(&text[last..]);
    (out, changed)
}

fn rewrite_token(
    token: &str,
    aliases: &HashMap<&str, &str>,
    api: &PytestPublicApi,
) -> Option<String> {
    let segments: Vec<&str> = token.split('.').collect();
    if segments.len() < 2 {
        return None;
    }
    let root = segments[0];
    let base = if root == PRIVATE_ROOT {
        PRIVATE_ROOT
    } else {
        aliases.get(root).copied()?
    };
    let name = segments[segments.len() - 1];
    let middle = &segments[1..segments.len() - 1];
    let module = if middle.is_empty() {
        base.to_string()
    } else {
        format!("{}.{}", base, middle.join("."))
    };
    let public = api.public_name(&module, name)?;
    Some(format!("pytest.{public}"))
}

// ─── FixtureDatabase integration ──────────────────────────────────────────────

impl FixtureDatabase {
    /// Learn the public-API mapping from `site_packages/pytest/__init__.py`.
    ///
    /// Silently keeps the previous mapping (if any) when the file is missing
    /// or yields no entries.
    pub(crate) fn load_pytest_public_api(&self, site_packages: &Path) {
        let init = site_packages.join("pytest").join("__init__.py");
        let Ok(source) = std::fs::read_to_string(&init) else {
            debug!("No readable {:?}; keeping _pytest imports as-is", init);
            return;
        };
        match PytestPublicApi::from_init_source(&source) {
            Some(api) => {
                info!(
                    "Loaded {} pytest public API mappings from {:?}",
                    api.len(),
                    init
                );
                *self.pytest_public_api.write().unwrap() = Some(Arc::new(api));
            }
            None => debug!("No _pytest re-exports found in {:?}", init),
        }
    }

    /// The public-API mapping learned from the venv's pytest, if any.
    pub fn pytest_public_api(&self) -> Option<Arc<PytestPublicApi>> {
        self.pytest_public_api.read().unwrap().clone()
    }

    /// Prepare a fixture's return type for display/insertion in a consumer
    /// file: first map private `_pytest` names to the public `pytest` API,
    /// then adapt to the consumer's existing import style.
    ///
    /// Use this instead of calling `adapt_type_for_consumer` directly.
    pub fn adapt_return_type_for_consumer(
        &self,
        return_type: &str,
        fixture_imports: &[TypeImportSpec],
        consumer_import_map: &HashMap<String, TypeImportSpec>,
    ) -> (String, Vec<TypeImportSpec>) {
        match self.pytest_public_api() {
            Some(api) => {
                let (return_type, fixture_imports) =
                    publicize_pytest_types(return_type, fixture_imports, &api);
                adapt_type_for_consumer(&return_type, &fixture_imports, consumer_import_map)
            }
            None => adapt_type_for_consumer(return_type, fixture_imports, consumer_import_map),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(check_name: &str, import_statement: &str) -> TypeImportSpec {
        TypeImportSpec {
            check_name: check_name.to_string(),
            import_statement: import_statement.to_string(),
        }
    }

    /// A representative `pytest/__init__.py`.
    const PUBLIC_INIT: &str = r#"
from _pytest.capture import CaptureFixture
from _pytest.config import Config
from _pytest.monkeypatch import MonkeyPatch
from _pytest.tmpdir import TempPathFactory
"#;

    fn api() -> PytestPublicApi {
        PytestPublicApi::from_init_source(PUBLIC_INIT).unwrap()
    }

    fn db_with_api() -> FixtureDatabase {
        let db = FixtureDatabase::new();
        *db.pytest_public_api.write().unwrap() = Some(Arc::new(api()));
        db
    }

    // ── from_init_source ─────────────────────────────────────────────────────

    const INIT: &str = r#"
from _pytest import __version__
from _pytest.capture import CaptureFixture
from _pytest.config import Config
from _pytest.config import hookimpl
from _pytest.debugging import pytestPDB as __pytestPDB
from _pytest.mark import MARK_GEN as mark
from _pytest.mark import HIDDEN
from .local import Relative
from other import NotPrivate
from _pytest.python import *

__all__ = [
    "CaptureFixture",
    "Config",
    "hookimpl",
    "mark",
]
"#;

    #[test]
    fn test_from_init_source_respects_all_and_aliases() {
        let api = PytestPublicApi::from_init_source(INIT).unwrap();
        assert_eq!(
            api.public_name("_pytest.capture", "CaptureFixture"),
            Some("CaptureFixture")
        );
        assert_eq!(api.public_name("_pytest.config", "Config"), Some("Config"));
        // Alias: public name differs from the private one.
        assert_eq!(api.public_name("_pytest.mark", "MARK_GEN"), Some("mark"));
        // Not in __all__.
        assert_eq!(api.public_name("_pytest.mark", "HIDDEN"), None);
        // Underscore-prefixed / dunder names are never public.
        assert_eq!(api.public_name("_pytest.debugging", "pytestPDB"), None);
        // Non-private and relative imports ignored.
        assert_eq!(api.public_name("other", "NotPrivate"), None);
        assert_eq!(api.public_name("local", "Relative"), None);
    }

    #[test]
    fn test_from_init_source_without_all_keeps_public_names() {
        let src = "from _pytest.capture import CaptureFixture\nfrom _pytest.x import _hidden\n";
        let api = PytestPublicApi::from_init_source(src).unwrap();
        assert_eq!(api.len(), 1);
        assert_eq!(
            api.public_name("_pytest.capture", "CaptureFixture"),
            Some("CaptureFixture")
        );
    }

    #[test]
    fn test_from_init_source_unparseable_or_empty_is_none() {
        assert!(PytestPublicApi::from_init_source("def (:").is_none());
        assert!(PytestPublicApi::from_init_source("import os\n").is_none());
    }

    // ── publicize: from-imports ──────────────────────────────────────────────

    #[test]
    fn test_publicize_from_import() {
        let specs = [spec(
            "MonkeyPatch",
            "from _pytest.monkeypatch import MonkeyPatch",
        )];
        let (rt, out) = publicize_pytest_types("MonkeyPatch", &specs, &api());
        assert_eq!(rt, "MonkeyPatch");
        assert_eq!(
            out,
            vec![spec("MonkeyPatch", "from pytest import MonkeyPatch")]
        );
    }

    #[test]
    fn test_publicize_from_import_in_generic() {
        let specs = [
            spec(
                "CaptureFixture",
                "from _pytest.capture import CaptureFixture",
            ),
            spec("Optional", "from typing import Optional"),
        ];
        let (rt, out) = publicize_pytest_types("Optional[CaptureFixture[str]]", &specs, &api());
        assert_eq!(rt, "Optional[CaptureFixture[str]]");
        assert_eq!(
            out,
            vec![
                spec("CaptureFixture", "from pytest import CaptureFixture"),
                spec("Optional", "from typing import Optional"),
            ]
        );
    }

    #[test]
    fn test_publicize_preserves_alias() {
        let specs = [spec(
            "CF",
            "from _pytest.capture import CaptureFixture as CF",
        )];
        let (rt, out) = publicize_pytest_types("CF[str]", &specs, &api());
        assert_eq!(rt, "CF[str]");
        assert_eq!(
            out,
            vec![spec("CF", "from pytest import CaptureFixture as CF")]
        );
    }

    #[test]
    fn test_publicize_leaves_unmapped_private_names() {
        let specs = [spec(
            "SubRequest",
            "from _pytest.fixtures import SubRequest",
        )];
        let (rt, out) = publicize_pytest_types("SubRequest", &specs, &api());
        assert_eq!(rt, "SubRequest");
        assert_eq!(out, specs.to_vec());
    }

    #[test]
    fn test_publicize_ignores_lookalike_and_other_modules() {
        let specs = [
            spec(
                "CaptureFixture",
                "from _pytestx.capture import CaptureFixture",
            ),
            spec("Path", "from pathlib import Path"),
        ];
        let (rt, out) = publicize_pytest_types("tuple[CaptureFixture, Path]", &specs, &api());
        assert_eq!(rt, "tuple[CaptureFixture, Path]");
        assert_eq!(out, specs.to_vec());
    }

    #[test]
    fn test_publicize_deduplicates() {
        let specs = [
            spec("Config", "from _pytest.config import Config"),
            spec("Config", "from pytest import Config"),
        ];
        let (_, out) = publicize_pytest_types("Config", &specs, &api());
        assert_eq!(out, vec![spec("Config", "from pytest import Config")]);
    }

    // ── publicize: dotted references ─────────────────────────────────────────

    #[test]
    fn test_publicize_dotted_bare_import() {
        let specs = [spec("_pytest", "import _pytest.capture")];
        let (rt, out) =
            publicize_pytest_types("_pytest.capture.CaptureFixture[str]", &specs, &api());
        assert_eq!(rt, "pytest.CaptureFixture[str]");
        assert_eq!(out, vec![spec("pytest", "import pytest")]);
    }

    #[test]
    fn test_publicize_dotted_aliased_import() {
        let specs = [spec("cap", "import _pytest.capture as cap")];
        let (rt, out) = publicize_pytest_types("cap.CaptureFixture[str]", &specs, &api());
        assert_eq!(rt, "pytest.CaptureFixture[str]");
        assert_eq!(out, vec![spec("pytest", "import pytest")]);
    }

    #[test]
    fn test_publicize_dotted_partially_mapped_keeps_private_import() {
        let specs = [spec("_pytest", "import _pytest.capture")];
        let (rt, out) = publicize_pytest_types(
            "tuple[_pytest.capture.CaptureFixture, _pytest.fixtures.SubRequest]",
            &specs,
            &api(),
        );
        assert_eq!(
            rt,
            "tuple[pytest.CaptureFixture, _pytest.fixtures.SubRequest]"
        );
        assert_eq!(
            out,
            vec![
                spec("_pytest", "import _pytest.capture"),
                spec("pytest", "import pytest"),
            ]
        );
    }

    #[test]
    fn test_publicize_dotted_unmapped_is_untouched() {
        let specs = [spec("_pytest", "import _pytest.fixtures")];
        let (rt, out) = publicize_pytest_types("_pytest.fixtures.SubRequest", &specs, &api());
        assert_eq!(rt, "_pytest.fixtures.SubRequest");
        assert_eq!(out, specs.to_vec());
    }

    #[test]
    fn test_publicize_mixed_from_and_dotted() {
        let specs = [
            spec("Config", "from _pytest.config import Config"),
            spec("_pytest", "import _pytest.capture"),
        ];
        let (rt, out) = publicize_pytest_types(
            "tuple[Config, _pytest.capture.CaptureFixture]",
            &specs,
            &api(),
        );
        assert_eq!(rt, "tuple[Config, pytest.CaptureFixture]");
        assert_eq!(
            out,
            vec![
                spec("Config", "from pytest import Config"),
                spec("pytest", "import pytest"),
            ]
        );
    }

    #[test]
    fn test_publicize_noop_without_private_specs() {
        let specs = [spec("Path", "from pathlib import Path")];
        let (rt, out) = publicize_pytest_types("Path", &specs, &api());
        assert_eq!(rt, "Path");
        assert_eq!(out, specs.to_vec());
    }

    // ── FixtureDatabase integration ──────────────────────────────────────────

    #[test]
    fn test_db_keeps_private_imports_without_loaded_api() {
        let db = FixtureDatabase::new();
        let specs = [spec(
            "MonkeyPatch",
            "from _pytest.monkeypatch import MonkeyPatch",
        )];
        let (rt, out) = db.adapt_return_type_for_consumer("MonkeyPatch", &specs, &HashMap::new());
        assert_eq!(rt, "MonkeyPatch");
        assert_eq!(out, specs.to_vec());
    }

    #[test]
    fn test_db_publicizes_with_loaded_api() {
        let db = db_with_api();
        let specs = [spec(
            "MonkeyPatch",
            "from _pytest.monkeypatch import MonkeyPatch",
        )];
        let (rt, out) = db.adapt_return_type_for_consumer("MonkeyPatch", &specs, &HashMap::new());
        assert_eq!(rt, "MonkeyPatch");
        assert_eq!(
            out,
            vec![spec("MonkeyPatch", "from pytest import MonkeyPatch")]
        );
    }

    #[test]
    fn test_db_adapts_to_consumer_with_bare_pytest_import() {
        let db = db_with_api();
        let specs = [spec(
            "MonkeyPatch",
            "from _pytest.monkeypatch import MonkeyPatch",
        )];
        let mut consumer = HashMap::new();
        consumer.insert("pytest".to_string(), spec("pytest", "import pytest"));
        let (rt, out) = db.adapt_return_type_for_consumer("MonkeyPatch", &specs, &consumer);
        assert_eq!(rt, "pytest.MonkeyPatch");
        assert!(out.is_empty(), "no import should remain: {:?}", out);
    }

    #[test]
    fn test_db_loads_mapping_from_site_packages() {
        let temp = tempfile::tempdir().unwrap();
        let pytest_dir = temp.path().join("pytest");
        std::fs::create_dir_all(&pytest_dir).unwrap();
        std::fs::write(
            pytest_dir.join("__init__.py"),
            "from _pytest.newmod import NewThing\n__all__ = [\"NewThing\"]\n",
        )
        .unwrap();

        let db = FixtureDatabase::new();
        db.load_pytest_public_api(temp.path());

        let specs = [spec("NewThing", "from _pytest.newmod import NewThing")];
        let (_, out) = db.adapt_return_type_for_consumer("NewThing", &specs, &HashMap::new());
        assert_eq!(out, vec![spec("NewThing", "from pytest import NewThing")]);
    }

    #[test]
    fn test_db_learns_nothing_when_init_missing() {
        let temp = tempfile::tempdir().unwrap();
        let db = FixtureDatabase::new();
        db.load_pytest_public_api(temp.path());
        assert!(db.pytest_public_api().is_none());
    }
}
