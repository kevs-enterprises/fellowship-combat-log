"""`parse_line` raises `ValueError` on a malformed line rather than returning a result-union."""

import pytest
from conftest import load_fixture

import fellowship_combat_log as fcl


def _lines():
    return [line for line in load_fixture("malformed.log").splitlines() if line]


@pytest.mark.parametrize("index", range(4))
def test_every_malformed_line_raises_value_error(index):
    line = _lines()[index]
    with pytest.raises(ValueError):
        fcl.parse_line(index + 1, line)


def test_empty_line_raises_value_error():
    with pytest.raises(ValueError, match="empty"):
        fcl.parse_line(1, "")


def test_error_message_mentions_the_seq_it_was_given():
    with pytest.raises(ValueError, match=r"\bline 42\b"):
        fcl.parse_line(42, "")


def test_a_well_formed_line_does_not_raise():
    # Sanity check the fixture actually contrasts with a good line, not just bad ones.
    good = load_fixture("damage.log").splitlines()[0]
    fcl.parse_line(1, good)  # must not raise
