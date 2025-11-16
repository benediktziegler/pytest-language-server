import pytest


@pytest.fixture
def sample_fixture():
    """A sample fixture that returns a value."""
    return 42


@pytest.fixture
def another_fixture():
    """Another fixture."""
    return "hello world"


@pytest.fixture
def cli_runner():
    """Parent fixture defined in root conftest.py"""
    return "parent_cli_runner"


@pytest.fixture
def database():
    """Database fixture that will be overridden in subdir"""
    return "parent_database"


@pytest.fixture
def shared_resource():
    """Shared resource used by multiple tests"""
    return {"status": "ready"}
