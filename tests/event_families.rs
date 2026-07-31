//! Per-family parse coverage: one synthetic fixture per core v8 event family
//! through the public parse surface, asserting the typed body and key field
//! mapping — never parser internals. Every fixture here is a committed synthetic
//! snippet.

use fellowship_combat_log::event::{
    CastPhase, DamageHealKind, EffectPhase, EncounterPhase, EventBody, Guid, Polarity, ResultTier,
};
use fellowship_combat_log::parse::parse_line;

/// Parse every non-empty line of a fixture through the public surface into bodies.
fn bodies(fixture: &str) -> Vec<EventBody> {
    fixture
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            parse_line(i as u32 + 1, line)
                .unwrap_or_else(|error| panic!("fixture line {} failed: {error:?}", i + 1))
                .body
        })
        .collect()
}

#[test]
fn damage_family() {
    let bodies = bodies(include_str!("fixtures/damage.log"));
    let EventBody::DamageHeal(damage) = &bodies[0] else {
        panic!("expected DamageHeal, got {:?}", bodies[0]);
    };
    assert_eq!(damage.kind, DamageHealKind::AbilityDamage);
    assert!(damage.kind.is_damage());
    assert_eq!(damage.source, Guid::Player(2_000_000_001));
    assert_eq!(damage.ability_id, 2669);
    assert_eq!(damage.applied, 112);
    assert_eq!(damage.absorbed, -5); // negative absorb = amplification
    assert_eq!(damage.overkill, -1); // -1 = none
    assert_eq!(damage.raw, 118);
    assert_eq!(damage.result, ResultTier::CriticalStrike);
}

#[test]
fn heal_family() {
    let bodies = bodies(include_str!("fixtures/heal.log"));
    let EventBody::DamageHeal(heal) = &bodies[0] else {
        panic!("expected DamageHeal, got {:?}", bodies[0]);
    };
    assert_eq!(heal.kind, DamageHealKind::Heal);
    assert!(!heal.kind.is_damage());
    assert_eq!(heal.applied, 450); // effective heal
    assert_eq!(heal.overkill, 2907); // overheal
    assert_eq!(heal.raw, 0); // raw is always 0 for heals
}

#[test]
fn spell_cast_pipeline() {
    let bodies = bodies(include_str!("fixtures/spell_cast.log"));
    let phases: Vec<&CastPhase> = bodies
        .iter()
        .map(|body| match body {
            EventBody::Cast(cast) => &cast.phase,
            other => panic!("expected Cast, got {other:?}"),
        })
        .collect();
    assert_eq!(phases[0], &CastPhase::Activated);
    assert_eq!(phases[1], &CastPhase::CastStart { cast_seconds: 1.5 });
    assert_eq!(
        phases[2],
        &CastPhase::CastFail {
            reason: "AbilityFailed.CastCancelled".to_string()
        }
    );
    // f5 ability id, f7 has-target flag, f8 target.
    let EventBody::Cast(fail) = &bodies[2] else {
        unreachable!()
    };
    assert_eq!(fail.ability_id, 1004);
    assert!(fail.has_target);
    assert_eq!(
        fail.target,
        Guid::Npc {
            spawn: 3_206_022_608,
            template: 41
        }
    );
}

#[test]
fn effect_aura_lifecycle() {
    let bodies = bodies(include_str!("fixtures/effect_aura.log"));
    let effects: Vec<_> = bodies
        .iter()
        .map(|body| match body {
            EventBody::Effect(effect) => effect,
            other => panic!("expected Effect, got {other:?}"),
        })
        .collect();
    assert_eq!(effects[0].phase, EffectPhase::Applied);
    assert_eq!(effects[0].polarity, Polarity::Debuff);
    assert_eq!(effects[0].duration_seconds, 8.0);
    assert_eq!(effects[0].granting_ability_id, 170);
    assert!(effects[0].refresher.is_none());

    assert_eq!(effects[1].phase, EffectPhase::Removed);
    assert_eq!(effects[1].duration_seconds, 0.0);

    assert_eq!(effects[2].phase, EffectPhase::Refreshed);
    assert_eq!(effects[2].stacks, 2);
    assert_eq!(
        effects[2].refresher,
        Some(Guid::Npc {
            spawn: 3_049_784_064,
            template: 42
        })
    );
}

#[test]
fn resource_changed_family() {
    let bodies = bodies(include_str!("fixtures/resource_changed.log"));
    let EventBody::ResourceChange(resource) = &bodies[0] else {
        panic!("expected ResourceChange, got {:?}", bodies[0]);
    };
    assert_eq!(resource.resource_type, 2);
    assert_eq!(resource.delta, 587.93);
    assert_eq!(resource.current, 587.93);
    assert_eq!(resource.max, 58792.80);
    assert_eq!(resource.causing_ability_id, 0);
}

#[test]
fn encounter_boundaries_with_comma_bearing_boss_names() {
    let bodies = bodies(include_str!("fixtures/encounter.log"));
    let EventBody::Encounter(start) = &bodies[0] else {
        panic!("expected Encounter, got {:?}", bodies[0]);
    };
    assert_eq!(start.phase, EncounterPhase::Start);
    assert_eq!(start.encounter_id, 30);
    // The nesting-aware tokenizer must not split the comma inside "Xul, The …".
    assert_eq!(
        start.bosses,
        vec![
            "Malgut the Fetid".to_string(),
            "Xul, The Blood Monolith".to_string()
        ]
    );

    let EventBody::Encounter(end) = &bodies[1] else {
        unreachable!()
    };
    assert_eq!(end.phase, EncounterPhase::End { success: false }); // a wipe
}

#[test]
fn combatant_info_top_level_framing() {
    let bodies = bodies(include_str!("fixtures/combatant_info.log"));
    let EventBody::CombatantInfo(info) = &bodies[0] else {
        panic!("expected CombatantInfo, got {:?}", bodies[0]);
    };
    assert_eq!(info.ulid, "01AAAAAAAAAAAAAAAAAAAAAAAA");
    assert_eq!(info.player, Guid::Player(1_000_000_001));
    assert!(info.is_recording_player);
    assert_eq!(info.hero_id, 10);
}

#[test]
fn unknown_event_types_are_surfaced_not_dropped() {
    let body = parse_line(1, "2026-07-22T10:29:06.540+02:00|SOME_FUTURE_EVENT|a|b|c")
        .expect("an unknown type still parses (timestamp is valid)")
        .body;
    assert_eq!(
        body,
        EventBody::Unknown {
            raw_type: "SOME_FUTURE_EVENT".to_string()
        }
    );
}
