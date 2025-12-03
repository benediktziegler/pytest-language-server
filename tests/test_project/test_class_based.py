"""Test file for class-based tests using fixtures."""

import pytest


@pytest.fixture
def shared_fixture() -> str:
    """A fixture shared between test methods in a class."""
    return "shared_value"


@pytest.fixture
def another_fixture() -> int:
    """Another fixture for testing."""
    return 42


class TestClassBased:
    """Test class with methods that use fixtures."""

    def test_uses_shared(self, shared_fixture: str):
        """Test method using shared_fixture."""
        assert shared_fixture == "shared_value"

    def test_uses_both(self, shared_fixture: str, another_fixture: int):
        """Test method using multiple fixtures."""
        assert shared_fixture == "shared_value"
        assert another_fixture == 42

    def test_uses_another(self, another_fixture: int):
        """Test method using another_fixture."""
        assert another_fixture == 42


class TestNestedClasses:
    """Outer test class."""

    def test_outer(self, shared_fixture: str):
        """Test in outer class."""
        assert shared_fixture == "shared_value"

    class TestInner:
        """Nested test class."""

        def test_inner(self, shared_fixture: str):
            """Test in nested class."""
            assert shared_fixture == "shared_value"
