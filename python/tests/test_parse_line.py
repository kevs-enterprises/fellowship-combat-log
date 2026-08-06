"""`parse_line` decodes one v8 log line into the JSON-shaped `Event` dict every binding shares."""

from conftest import load_fixture

import fellowship_combat_log as fcl


def _lines(fixture_name: str):
    return [line for line in load_fixture(fixture_name).splitlines() if line]


def test_damage_line_decodes_the_damage_heal_shape():
    (line,) = _lines("damage.log")
    event = fcl.parse_line(1, line)

    assert event["instant"]["seq"] == 1
    assert isinstance(event["instant"]["utc_ms"], int)

    body = event["body"]
    assert body["type"] == "DamageHeal"
    assert body["kind"] == "AbilityDamage"
    assert body["source"] == {"type": "Player", "id": 2000000001}
    assert body["target"] == {"type": "Npc", "spawn": 3049784064, "template": 42}
    assert body["school"] == "Magical"
    assert body["result"] == "CriticalStrike"
    assert body["applied"] == 112


def test_heal_line_decodes():
    (line,) = _lines("heal.log")
    body = fcl.parse_line(1, line)["body"]
    assert body["type"] == "DamageHeal"
    assert body["kind"] == "Heal"
    assert body["applied"] == 450


def test_cast_pipeline_phases_are_tagged_unions():
    activated, cast_start, cast_fail = _lines("spell_cast.log")

    activated_body = fcl.parse_line(1, activated)["body"]
    assert activated_body["type"] == "Cast"
    assert activated_body["phase"] == {"type": "Activated"}

    start_body = fcl.parse_line(2, cast_start)["body"]
    assert start_body["phase"]["type"] == "CastStart"
    assert start_body["phase"]["cast_seconds"] == 1.5

    fail_body = fcl.parse_line(3, cast_fail)["body"]
    assert fail_body["phase"]["type"] == "CastFail"
    assert fail_body["phase"]["reason"] == "AbilityFailed.CastCancelled"


def test_effect_phases_decode_and_refreshed_carries_a_refresher():
    applied, removed, refreshed = _lines("effect_aura.log")

    applied_body = fcl.parse_line(1, applied)["body"]
    assert applied_body["type"] == "Effect"
    assert applied_body["phase"] == "Applied"
    assert applied_body["polarity"] == "Debuff"
    assert applied_body["refresher"] is None

    removed_body = fcl.parse_line(2, removed)["body"]
    assert removed_body["phase"] == "Removed"

    refreshed_body = fcl.parse_line(3, refreshed)["body"]
    assert refreshed_body["phase"] == "Refreshed"
    assert refreshed_body["refresher"] == {
        "type": "Npc",
        "spawn": 3049784064,
        "template": 42,
    }


def test_resource_changed_decodes_scalars():
    (line,) = _lines("resource_changed.log")
    body = fcl.parse_line(1, line)["body"]
    assert body["type"] == "ResourceChange"
    assert body["source"] == {"type": "Player", "id": 1000000001}
    assert body["resource_type"] == 2
    assert body["current"] == 587.93


def test_encounter_start_and_wipe_end():
    start_line, end_line = _lines("encounter.log")

    start_body = fcl.parse_line(1, start_line)["body"]
    assert start_body["type"] == "Encounter"
    assert start_body["phase"] == {"type": "Start"}
    assert start_body["bosses"] == ["Malgut the Fetid", "Xul, The Blood Monolith"]

    end_body = fcl.parse_line(2, end_line)["body"]
    assert end_body["phase"] == {"type": "End", "success": False}


def test_combatant_info_top_level_framing():
    (line,) = _lines("combatant_info.log")
    body = fcl.parse_line(1, line)["body"]

    assert body["type"] == "CombatantInfo"
    assert body["ulid"] == "01AAAAAAAAAAAAAAAAAAAAAAAA"
    assert body["player"] == {"type": "Player", "id": 1000000001}
    assert body["is_recording_player"] is True
    assert body["hero_id"] == 10


def test_environment_and_unrecognized_guid_namespaces():
    # ABILITY_ACTIVATED's target namespace in this fixture is UnrecognizedType-0.
    (line, *_rest) = _lines("spell_cast.log")
    body = fcl.parse_line(1, line)["body"]
    assert body["target"] == {"type": "Unrecognized"}


def test_an_unrecognized_event_type_decodes_as_unknown():
    body = fcl.parse_line(1, "2026-07-22T10:29:06.540+02:00|SOME_FUTURE_EVENT|1|2|3")["body"]
    assert body == {
        "type": "Unknown",
        "raw_type": "SOME_FUTURE_EVENT",
        "raw_fields": ["1", "2", "3"],
    }


def test_version_is_a_nonempty_string():
    version = fcl.version()
    assert isinstance(version, str)
    assert version
