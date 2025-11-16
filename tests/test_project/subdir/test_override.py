def test_overridden_fixture(sample_fixture):
    """Test that should use the overridden fixture (42 + 100 = 142)."""
    assert sample_fixture == 142


def test_local_fixture(local_fixture):
    """Test using the local fixture."""
    assert local_fixture == "local"


def test_both_fixtures(sample_fixture, local_fixture):
    """Test using both overridden and local fixtures."""
    assert sample_fixture == 142
    assert local_fixture == "local"
