import pytest


@pytest.fixture
def sample_fixture(sample_fixture):
    """Override parent fixture, adding 100 to the value."""
    return sample_fixture + 100


@pytest.fixture
def local_fixture():
    """A fixture local to this subdirectory."""
    return "local"
