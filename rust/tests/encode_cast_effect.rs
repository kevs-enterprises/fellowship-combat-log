//! Round-trip and byte-exact proof for the cast/channel pipeline (7 tokens),
//! `EFFECT_*` (3 tokens), `ABILITY_DISPEL`, and `RESOURCE_CHANGED` — 12
//! tokens, the two richest state-machine-shaped families in the catalog.
//!
//! `Cast.resources` and `Effect.refresher`/`phase` consistency are both
//! untested by the fixture (every fixture cast has empty `resources`; no
//! fixture line exercises the mismatch), so both get dedicated hand-built
//! cases here.

mod common;

use common::{assert_family_round_trips, body_of, event_with};
use fellowship_combat_log::encode::{EncodeError, encode_line};
use fellowship_combat_log::event::{
    Cast, CastPhase, Dispel, Effect, EffectPhase, EventBody, Guid, Polarity, ResourceChange,
};
use fellowship_combat_log::parse::parse_line;

fn is_cast_effect_family(body: &EventBody) -> bool {
    matches!(
        body,
        EventBody::Cast(_)
            | EventBody::Effect(_)
            | EventBody::Dispel(_)
            | EventBody::ResourceChange(_)
    )
}

#[test]
fn all_twelve_tokens_round_trip_from_the_fixture() {
    // 7 cast/channel + 3 effect + DISPEL + RESOURCE_CHANGED = 12 fixture lines.
    assert_family_round_trips(is_cast_effect_family, 12);
}

#[test]
fn a_cast_with_non_empty_resources_round_trips() {
    let event = event_with(EventBody::Cast(Cast {
        phase: CastPhase::CastSuccess,
        caster: Guid::Player(2000000001),
        ability_id: 1313,
        has_target: true,
        target: Guid::Npc {
            spawn: 3320316320,
            template: 173,
        },
        resources: vec![(2, 100.0, 100.0), (5, 42.5, 60.0)],
    }));
    let encoded = encode_line(&event).unwrap();
    let body = body_of(&encoded);
    assert!(
        body.contains("[(2,100,100),(5,42.5,60)]"),
        "expected the real resources tuple list in {body:?}"
    );
    let reparsed = parse_line(1, &encoded).unwrap();
    assert_eq!(reparsed, event);
}

#[test]
fn cast_activated_wire_form_is_exact_at_sixteen_fields() {
    let event = event_with(EventBody::Cast(Cast {
        phase: CastPhase::Activated,
        caster: Guid::Player(1),
        ability_id: 10,
        has_target: false,
        target: Guid::Player(2),
        resources: vec![],
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "ABILITY_ACTIVATED|Player-1|\"-\"|10|\"-\"|0|Player-2|\"0\"|0|0|0|0.0|0.0|0.0|[]"
    );
}

#[test]
fn cast_start_appends_a_shortest_form_cast_time_as_a_seventeenth_field() {
    let event = event_with(EventBody::Cast(Cast {
        phase: CastPhase::CastStart { cast_seconds: 1.5 },
        caster: Guid::Player(1),
        ability_id: 10,
        has_target: false,
        target: Guid::Player(2),
        resources: vec![],
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "ABILITY_CAST_START|Player-1|\"-\"|10|\"-\"|0|Player-2|\"0\"|0|0|0|0.0|0.0|0.0|[]|1.5"
    );
}

#[test]
fn channel_fail_appends_a_quoted_reason_as_a_seventeenth_field() {
    let event = event_with(EventBody::Cast(Cast {
        phase: CastPhase::ChannelFail {
            reason: "AbilityFailed.CastCancelled".to_string(),
        },
        caster: Guid::Player(1),
        ability_id: 10,
        has_target: false,
        target: Guid::Player(2),
        resources: vec![],
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "ABILITY_CHANNEL_FAIL|Player-1|\"-\"|10|\"-\"|0|Player-2|\"0\"|0|0|0|0.0|0.0|0.0|[]|\"AbilityFailed.CastCancelled\""
    );
}

#[test]
fn effect_applied_wire_form_is_exact_at_twenty_one_fields() {
    let event = event_with(EventBody::Effect(Effect {
        phase: EffectPhase::Applied,
        caster: Guid::Player(1),
        target: Guid::Player(2),
        effect_id: 101,
        duration_seconds: 8.0,
        stacks: 1,
        polarity: Polarity::Debuff,
        granting_ability_id: 170,
        refresher: None,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "EFFECT_APPLIED|Player-1|\"-\"|Player-2|\"-\"|101|\"-\"|8|1|DEBUFF|0|0|0|0.0|0.0|0.0|[]|170|\"-\"|0"
    );
}

#[test]
fn effect_refreshed_wire_form_is_exact_at_twenty_three_fields() {
    let event = event_with(EventBody::Effect(Effect {
        phase: EffectPhase::Refreshed,
        caster: Guid::Player(2000000001),
        target: Guid::Player(2000000001),
        effect_id: 2649,
        duration_seconds: 30.0,
        stacks: 2,
        polarity: Polarity::Buff,
        granting_ability_id: 0,
        refresher: Some(Guid::Npc {
            spawn: 3049784064,
            template: 42,
        }),
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "EFFECT_REFRESHED|Player-2000000001|\"-\"|Player-2000000001|\"-\"|2649|\"-\"|30|2|BUFF|0|0|0|0.0|0.0|0.0|[]|0|\"-\"|0|Npc-3049784064-42|\"-\""
    );
}

#[test]
fn a_refreshed_phase_with_no_refresher_is_rejected_rather_than_emitting_a_short_line() {
    let event = event_with(EventBody::Effect(Effect {
        phase: EffectPhase::Refreshed,
        caster: Guid::Player(1),
        target: Guid::Player(2),
        effect_id: 1,
        duration_seconds: 1.0,
        stacks: 1,
        polarity: Polarity::Buff,
        granting_ability_id: 0,
        refresher: None,
    }));
    assert_eq!(
        encode_line(&event),
        Err(EncodeError::InconsistentState {
            reason: "phase is Refreshed but refresher is None",
        })
    );
}

#[test]
fn a_non_refreshed_phase_with_a_refresher_is_rejected_rather_than_silently_dropping_it() {
    let event = event_with(EventBody::Effect(Effect {
        phase: EffectPhase::Applied,
        caster: Guid::Player(1),
        target: Guid::Player(2),
        effect_id: 1,
        duration_seconds: 1.0,
        stacks: 1,
        polarity: Polarity::Buff,
        granting_ability_id: 0,
        refresher: Some(Guid::Player(3)),
    }));
    assert_eq!(
        encode_line(&event),
        Err(EncodeError::InconsistentState {
            reason: "refresher is Some but phase is not Refreshed",
        })
    );
}

#[test]
fn dispel_wire_form_is_exact() {
    let event = event_with(EventBody::Dispel(Dispel {
        dispeller: Guid::Player(3000000002),
        target: Guid::Player(1000000003),
        dispel_ability_id: 997,
        removed_effect_id: 946,
        remaining_seconds: 14.894555,
        polarity: Polarity::Debuff,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "ABILITY_DISPEL|Player-3000000002|\"-\"|Player-1000000003|\"-\"|997|\"-\"|946|\"-\"|14.894555|DEBUFF"
    );
}

#[test]
fn resource_changed_wire_form_keeps_max_and_the_unmodeled_float_distinct() {
    let event = event_with(EventBody::ResourceChange(ResourceChange {
        source: Guid::Player(1000000001),
        owner: Guid::Player(1000000001),
        resource_type: 2,
        delta: 587.93,
        current: 587.93,
        max: 58792.80,
        causing_ability_id: 0,
    }));
    let body = body_of(&encode_line(&event).unwrap()).to_string();
    let fields: Vec<&str> = body.split('|').collect();
    // 0=token,1=source,2=name,3=owner,4=name,5=type,6=delta,7=current,
    // 8=max(f10),9=unmodeled(f11),10=causing_ability_id(f12),11=name(f13).
    assert_eq!(fields[8], "58792.8", "f10 must be the real max");
    assert_eq!(
        fields[9], "0.0",
        "f11 is a separate unmodeled float, not a repeat of max"
    );
    let reparsed = parse_line(1, &encode_line(&event).unwrap()).unwrap();
    assert_eq!(reparsed, event);
}
