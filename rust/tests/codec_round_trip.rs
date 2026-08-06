//! Round-trip proof for the bidirectional wire codecs colocated with their types
//! in `event.rs`: `Guid`, `School`, `ResultTier`, and `Polarity` each parse and
//! render, and rendering then re-parsing must reproduce the original value.

use fellowship_combat_log::event::{
    Guid, Polarity, ResultTier, School, parse_guid, parse_polarity, parse_result, parse_school,
};

#[test]
fn guid_round_trips_across_all_four_namespaces() {
    let guids = [
        Guid::Player(1000000001),
        Guid::Player(0),
        Guid::Npc {
            spawn: 3049784064,
            template: 42,
        },
        Guid::Environment,
        Guid::Unrecognized,
    ];
    for guid in guids {
        let rendered = guid.render();
        assert_eq!(
            parse_guid(&rendered),
            Some(guid.clone()),
            "guid {guid:?} rendered to {rendered:?} did not round-trip"
        );
    }
}

#[test]
fn guid_wire_forms_are_exact() {
    assert_eq!(Guid::Player(1000000001).render(), "Player-1000000001");
    assert_eq!(
        Guid::Npc {
            spawn: 3049784064,
            template: 42
        }
        .render(),
        "Npc-3049784064-42"
    );
    assert_eq!(Guid::Environment.render(), "Environment-0");
    assert_eq!(Guid::Unrecognized.render(), "UnrecognizedType-0");
}

#[test]
fn school_wire_forms_are_exact() {
    // A matched-pair rename (render and parse_school agreeing on a *wrong* token)
    // would still pass the round-trip test below, so pin the actual wire text too.
    assert_eq!(School::Physical.render(), "Physical");
    assert_eq!(School::Magical.render(), "Magical");
    assert_eq!(School::None.render(), "None");
}

#[test]
fn result_tier_wire_forms_are_exact() {
    assert_eq!(ResultTier::Hit.render(), "Hit");
    assert_eq!(ResultTier::CriticalStrike.render(), "CriticalStrike");
    assert_eq!(
        ResultTier::GrievousCriticalStrike.render(),
        "GrievousCriticalStrike"
    );
    assert_eq!(ResultTier::Block.render(), "Block");
    assert_eq!(ResultTier::Parry.render(), "Parry");
    assert_eq!(ResultTier::Dodge.render(), "Dodge");
    assert_eq!(ResultTier::Miss.render(), "Miss");
    assert_eq!(ResultTier::None.render(), "None");
}

#[test]
fn school_round_trips_for_every_variant() {
    for school in [School::Physical, School::Magical, School::None] {
        let rendered = school.render();
        assert_eq!(
            parse_school(rendered),
            Some(school),
            "school {school:?} rendered to {rendered:?} did not round-trip"
        );
    }
}

#[test]
fn result_tier_round_trips_for_every_variant() {
    for result in [
        ResultTier::Hit,
        ResultTier::CriticalStrike,
        ResultTier::GrievousCriticalStrike,
        ResultTier::Block,
        ResultTier::Parry,
        ResultTier::Dodge,
        ResultTier::Miss,
        ResultTier::None,
    ] {
        let rendered = result.render();
        assert_eq!(
            parse_result(rendered),
            Some(result),
            "result tier {result:?} rendered to {rendered:?} did not round-trip"
        );
    }
}

#[test]
fn polarity_round_trips_for_both_variants() {
    for polarity in [Polarity::Buff, Polarity::Debuff] {
        let rendered = polarity.render();
        assert_eq!(
            parse_polarity(rendered),
            Some(polarity),
            "polarity {polarity:?} rendered to {rendered:?} did not round-trip"
        );
    }
}

#[test]
fn polarity_wire_form_is_uppercase() {
    assert_eq!(Polarity::Buff.render(), "BUFF");
    assert_eq!(Polarity::Debuff.render(), "DEBUFF");
}
