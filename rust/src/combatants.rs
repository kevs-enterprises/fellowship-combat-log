//! Listing the combatants in a log and their equipped gear.
//!
//! This scan is deliberately cheap: it needs only the `COMBATANT_INFO` lines, so it skips any
//! per-event fold and touches nothing else in a session log that may run to gigabytes. That
//! difference is the whole point — a caller answering a user interaction cannot wait for a full
//! aggregation pass.
//!
//! Ids cross this boundary raw. Resolving them against a catalog is the caller's job, since the
//! id spaces are per-namespace and the catalog is the consumer's to own.

use crate::event::{CombatantInfo, EventBody};
use crate::parse::parse_line;

/// One combatant found in a log, with the gear snapshot a caller reconstructs from.
#[derive(Clone, PartialEq, Debug)]
pub struct Combatant {
    /// Stable across a session — the same character re-appears under this ulid every encounter.
    pub ulid: String,
    /// The in-game character name as the log records it.
    pub name: String,
    /// True for the player whose client wrote the log, so the caller can default to them.
    pub is_recording_player: bool,
    /// The latest snapshot seen for this combatant: a session logs one per encounter, and the
    /// most recent reflects how they are geared *now* rather than however they started.
    pub info: CombatantInfo,
    /// Snapshots seen for this combatant across the session.
    pub snapshot_count: u32,
}

/// Every combatant a log mentions, in first-seen order, each carrying its latest gear snapshot.
///
/// Malformed or unparseable lines are skipped rather than failing the scan — a log is a live
/// recording that can be truncated mid-write, and one bad line must not cost the whole import.
pub fn list_combatants(log: &str) -> Vec<Combatant> {
    let mut found: Vec<Combatant> = Vec::new();
    for (index, line) in log.lines().enumerate() {
        // Cheap reject first: only a vanishing fraction of a session log is COMBATANT_INFO, so
        // the substring test is what keeps a multi-gigabyte scan from parsing every line.
        if !line.contains("COMBATANT_INFO") {
            continue;
        }
        let Ok(event) = parse_line(index as u32 + 1, line) else {
            continue;
        };
        let EventBody::CombatantInfo(info) = event.body else {
            continue;
        };
        let name = extract_name(line).unwrap_or_default();
        match found.iter_mut().find(|c| c.ulid == info.ulid) {
            Some(existing) => {
                existing.snapshot_count += 1;
                // Sticky: the flag is a property of the log's owner, so one sighting settles it.
                existing.is_recording_player |= info.is_recording_player;
                // A later snapshot can supply a name an earlier truncated one lacked; never let
                // a well-formed name be replaced by an empty one.
                if !name.is_empty() {
                    existing.name = name;
                }
                existing.info = info;
            }
            None => found.push(Combatant {
                ulid: info.ulid.clone(),
                name,
                is_recording_player: info.is_recording_player,
                info,
                snapshot_count: 1,
            }),
        }
    }
    found
}

/// The quoted character name (f5). The parser drops it — it is identity, not build context —
/// but the picker has to show the player something they recognise.
fn extract_name(line: &str) -> Option<String> {
    let field = line.split('|').nth(4)?;
    Some(field.trim().trim_matches('"').to_string())
}
