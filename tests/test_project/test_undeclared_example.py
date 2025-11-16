"""
Example test file demonstrating undeclared fixture detection.

This file intentionally has fixtures used in function bodies without
declaring them as parameters, to test the diagnostic and code action features.
"""


def test_with_undeclared_fixture():
    """
    This test uses sample_fixture without declaring it as a parameter.
    The LSP should warn about this and offer a code action to fix it.
    """
    # This should trigger a warning: sample_fixture used but not declared
    result = sample_fixture
    assert result == 42


def test_properly_declared(sample_fixture):
    """
    This test properly declares sample_fixture as a parameter.
    No warning should be shown.
    """
    result = sample_fixture
    assert result == 42


def test_multiple_undeclared():
    """
    This test uses multiple fixtures without declaring them.
    Should show warnings for both.
    """
    # Both should trigger warnings
    result1 = sample_fixture
    result2 = another_fixture
    assert result1 == 42
    assert result2 == "hello world"


def test_mixed_declared_undeclared(sample_fixture):
    """
    This test declares one fixture but uses another without declaring it.
    """
    # sample_fixture is declared, so no warning
    result1 = sample_fixture

    # another_fixture is NOT declared, should warn
    result2 = another_fixture

    assert result1 == 42
    assert result2 == "hello world"
