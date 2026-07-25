//! Gear decode from `COMBATANT_INFO` (#179): the equipped pieces, the realized trait ranks,
//! and the neck-trait picks a build is reconstructed from, plus the combatant listing the
//! import's picker is built on.
//!
//! The fixture is a redacted capture, not an invented one: real v8 structure and real game ids
//! — so the decode is exercised against the shape the game actually writes — with every
//! identity field (name, player id, ULID) replaced. Like every fixture here it is committed;
//! nothing reads `unpublished logs`.

use fellowship_combat_log::combatants::list_combatants;
use fellowship_combat_log::event::EventBody;
use fellowship_combat_log::parse::parse_line;

const FIXTURE: &str = include_str!("fixtures/combatant_info_v8.log");

fn first_info() -> fellowship_combat_log::event::CombatantInfo {
    let line = FIXTURE.lines().next().expect("fixture has a line");
    let event = parse_line(1, line).expect("fixture line parses");
    match event.body {
        EventBody::CombatantInfo(info) => info,
        other => panic!("expected COMBATANT_INFO, got {other:?}"),
    }
}

#[test]
fn decodes_one_piece_per_gear_slot() {
    let info = first_info();
    assert_eq!(info.gear.len(), 14, "one entry per gear slot");
    assert!(
        info.gear
            .iter()
            .all(|p| p.as_ref().is_some_and(|p| p.item_id > 0)),
        "every slot decodes to an item"
    );
}

#[test]
fn decodes_a_pieces_item_stats_and_score() {
    let info = first_info();
    let head = info.gear[0].as_ref().expect("head decodes");
    assert_eq!(head.item_id, 5213);
    assert_eq!(head.item_level, 315);
    // Fixed slots first, then the rolls — the leading entries are the item's own template.
    assert_eq!(head.stats[0], (1, 34));
    assert_eq!(head.stats[1], (3, 17));
    assert_eq!(head.stats[2], (23, 18));
    assert!(head.score > 0.0);
}

#[test]
fn a_set_bonus_is_carried_by_exactly_two_pieces() {
    let info = first_info();
    let mut carriers = std::collections::BTreeMap::<u32, u32>::new();
    for piece in info.gear.iter().flatten() {
        if let Some(set) = piece.set_bonus_id {
            *carriers.entry(set).or_default() += 1;
        }
    }
    assert!(!carriers.is_empty(), "the fixture runs sets");
    for (set, count) in &carriers {
        assert_eq!(*count, 2, "set {set} should activate across two carriers");
    }
}

#[test]
fn separates_ability_grants_from_traits() {
    let info = first_info();
    // The two nested lists index different namespaces; conflating them mis-resolves gear.
    let abilities: Vec<u32> = info
        .gear
        .iter()
        .flatten()
        .flat_map(|p| p.ability_grants.iter().map(|(id, _)| *id))
        .collect();
    let traits: Vec<u32> = info
        .gear
        .iter()
        .flatten()
        .flat_map(|p| p.traits.iter().map(|(id, _)| *id))
        .collect();
    assert!(!abilities.is_empty(), "the fixture grants abilities");
    assert!(!traits.is_empty(), "the fixture carries traits");
    // Every per-piece trait grant is reflected in the realized ranks.
    for id in &traits {
        assert!(
            info.trait_ranks.iter().any(|(t, _)| t == id),
            "trait {id} missing from the realized ranks"
        );
    }
}

#[test]
fn decodes_gem_attunements() {
    let info = first_info();
    let gems: Vec<(u32, u32)> = info
        .gear
        .iter()
        .flatten()
        .flat_map(|p| p.gems.clone())
        .collect();
    assert!(!gems.is_empty(), "the fixture socket gems");
    assert!(
        gems.iter().all(|(_, power)| *power == 100),
        "an attunement is always +100 power"
    );
}

#[test]
fn decodes_the_neck_trait_picks() {
    let info = first_info();
    assert_eq!(info.neck_traits.len(), 4, "four candidates are offered");
    let chosen = info.neck_traits.iter().filter(|c| c.selected).count();
    assert_eq!(chosen, 2, "exactly two are picked");
}

#[test]
fn identifies_the_legendary_by_its_higher_item_level() {
    let info = first_info();
    let baseline = info.gear[0].as_ref().expect("head decodes").item_level;
    let odd: Vec<_> = info
        .gear
        .iter()
        .flatten()
        .filter(|p| p.item_level != baseline)
        .collect();
    assert_eq!(odd.len(), 1, "exactly one piece differs");
    let legendary = odd[0];
    assert!(legendary.set_bonus_id.is_none());
    assert!(
        legendary.traits.is_empty(),
        "a legendary holds no modifiers"
    );
}

#[test]
fn lists_every_combatant_and_flags_the_recording_player() {
    let found = list_combatants(FIXTURE);
    assert_eq!(found.len(), 2, "the fixture holds two combatants");
    let recording: Vec<_> = found.iter().filter(|c| c.is_recording_player).collect();
    assert_eq!(recording.len(), 1, "exactly one wrote the log");
    assert_eq!(recording[0].name, "P1");
    assert!(found.iter().all(|c| c.info.gear.len() == 14));
}

#[test]
fn keeps_the_latest_snapshot_per_combatant() {
    // A session logs one snapshot per encounter, so the newest must win — a player who re-geared
    // mid-session should import what they finished in, not what they started in.
    let regeared = FIXTURE.replace("(5213,315,", "(5287,315,");
    assert_ne!(
        regeared, FIXTURE,
        "the rewrite must actually change the gear"
    );
    let found = list_combatants(&format!("{FIXTURE}{regeared}"));
    assert_eq!(found.len(), 2, "the same character collapses to one entry");
    let pov = found
        .iter()
        .find(|c| c.is_recording_player)
        .expect("a recording player");
    assert_eq!(pov.snapshot_count, 2);
    assert_eq!(
        pov.info.gear[0].as_ref().expect("head decodes").item_id,
        5287,
        "the later snapshot's gear is the one kept"
    );
}

#[test]
fn an_unreadable_piece_holds_its_slot_rather_than_shifting_the_rest() {
    // Slot identity is positional, so dropping a bad entry would silently move every later
    // piece onto the wrong slot — a build wearing gear it never had.
    let damaged = FIXTURE.replacen("(5213,315,", "(nonsense,315,", 1);
    let event = parse_line(1, damaged.lines().next().expect("a line")).expect("still parses");
    let EventBody::CombatantInfo(info) = event.body else {
        panic!("expected COMBATANT_INFO");
    };
    assert_eq!(info.gear.len(), 14, "the slot count is unchanged");
    assert!(info.gear[0].is_none(), "the damaged slot reads as unknown");
    assert_eq!(
        info.gear[1].as_ref().expect("necklace decodes").item_id,
        5244,
        "later slots keep their own pieces"
    );
}

#[test]
fn a_line_truncated_before_the_gear_still_yields_its_identity() {
    // The sim resolves its point-of-view from these lines, so a recording truncated mid-write
    // must not cost the whole extract.
    let full = FIXTURE.lines().next().expect("a line");
    let truncated: String = full.split('|').take(11).collect::<Vec<_>>().join("|");
    let event = parse_line(1, &truncated).expect("a short line still parses");
    let EventBody::CombatantInfo(info) = event.body else {
        panic!("expected COMBATANT_INFO");
    };
    assert!(info.is_recording_player, "identity survives");
    assert_eq!(info.hero_id, 7);
    assert!(info.gear.is_empty(), "with no gear to report");
}

#[test]
fn skips_malformed_lines_rather_than_failing_the_scan() {
    let corrupted = format!("{FIXTURE}not|a|real|line\n\n");
    let found = list_combatants(&corrupted);
    assert_eq!(found.len(), 2, "a truncated tail costs nothing");
}
