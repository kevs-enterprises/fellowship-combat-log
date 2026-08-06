"""`list_combatants` scans a whole log for every character it mentions."""

from conftest import load_fixture

import fellowship_combat_log as fcl


def test_finds_the_one_combatant_in_the_simple_fixture():
    log = load_fixture("combatant_info.log")
    combatants = fcl.list_combatants(log)

    assert len(combatants) == 1
    combatant = combatants[0]
    assert combatant["ulid"] == "01AAAAAAAAAAAAAAAAAAAAAAAA"
    assert combatant["name"] == "P1"
    assert combatant["is_recording_player"] is True
    assert combatant["snapshot_count"] == 1
    assert combatant["info"]["hero_id"] == 10


def test_finds_every_combatant_and_flags_the_recording_player():
    log = load_fixture("combatant_info_v8.log")
    combatants = fcl.list_combatants(log)

    assert len(combatants) == 2, "the fixture holds two combatants"
    recording = [c for c in combatants if c["is_recording_player"]]
    assert len(recording) == 1, "exactly one wrote the log"
    assert recording[0]["name"] == "P1"
    assert all(len(c["info"]["gear"]) == 14 for c in combatants)


def test_gear_pieces_decode_to_the_gear_piece_shape():
    log = load_fixture("combatant_info_v8.log")
    combatants = fcl.list_combatants(log)
    head = combatants[0]["info"]["gear"][0]

    assert head is not None
    assert head["item_id"] > 0
    assert isinstance(head["stats"], list)
    assert all(len(pair) == 2 for pair in head["stats"])
    assert isinstance(head["score"], float)


def test_never_raises_on_a_log_with_malformed_lines():
    log = load_fixture("combatant_info_v8.log") + "not|a|real|line\n\n"
    combatants = fcl.list_combatants(log)  # must not raise
    assert len(combatants) == 2, "a truncated tail costs nothing"


def test_a_log_with_no_combatants_yields_an_empty_list():
    assert fcl.list_combatants("") == []
    assert fcl.list_combatants(load_fixture("damage.log")) == []
