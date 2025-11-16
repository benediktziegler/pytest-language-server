import pytest


def test_uses_overridden_cli_runner(cli_runner):
    """Test uses the overridden cli_runner (same as parent in this case)"""
    assert cli_runner == "parent_cli_runner"


def test_uses_overridden_database(database):
    """Test uses the overridden database fixture"""
    assert "parent_database" in database
    assert "modified" in database
    assert "ready" in database


def test_uses_inherited_shared_resource(shared_resource):
    """Test uses shared_resource from parent (not overridden)"""
    assert shared_resource["status"] == "ready"


def test_uses_local_fixture(local_fixture):
    """Test uses local fixture defined only in subdir"""
    assert local_fixture == "local"


def test_multiple_fixtures(cli_runner, database, shared_resource, local_fixture):
    """Test uses a mix of overridden, inherited, and local fixtures"""
    assert cli_runner == "parent_cli_runner"
    assert "parent_database" in database
    assert shared_resource["status"] == "ready"
    assert local_fixture == "local"


def test_second_usage(cli_runner):
    """Another test using cli_runner"""
    assert cli_runner is not None


def test_third_usage(cli_runner, database):
    """Third test using both cli_runner and database"""
    assert cli_runner == "parent_cli_runner"
    assert "parent_database" in database
