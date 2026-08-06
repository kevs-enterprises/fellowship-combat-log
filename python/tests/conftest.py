"""Shared pytest fixtures for the `fellowship_combat_log` extension's tests.

The fixture logs are the same ones the Rust crate's own tests decode — this package
reimplements no parsing, so testing against a second copy of the corpus would test nothing new.
"""

from pathlib import Path

import pytest

FIXTURES_DIR = Path(__file__).resolve().parent.parent.parent / "rust" / "tests" / "fixtures"


@pytest.fixture
def fixtures_dir() -> Path:
    return FIXTURES_DIR


def load_fixture(name: str) -> str:
    return (FIXTURES_DIR / name).read_text()
