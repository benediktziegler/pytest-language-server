"""Tests using keyword-only and positional-only fixture arguments.

These tests verify that fixtures can be used with Python 3.8+ syntax:
- Keyword-only arguments (after *)
- Positional-only arguments (before /)
- Mixed argument types
"""

from pathlib import Path


def test_keyword_only_fixture(*, sample_fixture: int) -> None:
    """Test using a fixture as keyword-only argument."""
    assert sample_fixture == 42


def test_keyword_only_with_type_annotation(*, another_fixture: str) -> None:
    """Test using a fixture as keyword-only argument with type annotation."""
    assert another_fixture == "hello world"


def test_positional_only_fixture(sample_fixture: int, /) -> None:
    """Test using a fixture as positional-only argument."""
    assert sample_fixture == 42


def test_mixed_fixture_args(
    sample_fixture: int,
    /,
    another_fixture: str,
    *,
    shared_resource: dict,
) -> None:
    """Test using fixtures with all argument types: positional-only, regular, and keyword-only."""
    assert sample_fixture == 42
    assert another_fixture == "hello world"
    assert shared_resource == {"status": "ready"}


def test_multiple_keyword_only_fixtures(
    *, sample_fixture: int, another_fixture: str, shared_resource: dict
) -> None:
    """Test using multiple keyword-only fixtures."""
    assert sample_fixture == 42
    assert another_fixture == "hello world"
    assert shared_resource == {"status": "ready"}
