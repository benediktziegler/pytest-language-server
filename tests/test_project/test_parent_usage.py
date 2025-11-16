import pytest


def test_uses_parent_cli_runner(cli_runner):
    """Test that uses the parent cli_runner fixture"""
    assert cli_runner == "parent_cli_runner"


def test_uses_parent_database(database):
    """Test that uses the parent database fixture"""
    assert database == "parent_database"


def test_uses_shared_resource(shared_resource):
    """Test that uses shared_resource fixture"""
    assert shared_resource["status"] == "ready"


def test_multiple_parent_fixtures(cli_runner, database, shared_resource):
    """Test that uses multiple parent fixtures"""
    assert cli_runner == "parent_cli_runner"
    assert database == "parent_database"
    assert shared_resource["status"] == "ready"
