//! Worked example: a consumer decoding and re-encoding an event type this
//! crate has never heard of, using only public API — the pattern DR-0003
//! opens up. `EventBody::Unknown` keeps a line's raw fields instead of
//! discarding them, and the tokenizing/quoting/rendering primitives the
//! built-in families are built from are `pub`, so a consumer's own event type
//! doesn't need to re-derive the wire format's quoting and bracket-nesting
//! rules from scratch. This is also a real regression test: if one of the
//! primitives it calls stops being `pub`, or the round trip below breaks,
//! this file fails to compile or fails to pass.

use fellowship_combat_log::encode::{
    EncodeError, encode_line, render_float, render_int_array, render_quoted,
};
use fellowship_combat_log::event::{EventBody, Guid, parse_guid};
use fellowship_combat_log::parse::{parse_int_array, parse_line, unquote};
use fellowship_combat_log::timestamp::{LogInstant, render_instant};

/// A made-up event type: a raid leader's pull timer, naming the caller, the
/// countdown, a label, and the participant ids. Not part of the v8 catalog —
/// exactly the shape of type a consumer would define for their own extension.
#[derive(Debug, PartialEq)]
struct PullTimer {
    caller: Guid,
    seconds: f64,
    label: String,
    participants: Vec<u32>,
}

impl PullTimer {
    const TYPE: &'static str = "CUSTOM_PULL_TIMER";

    /// Decode from `EventBody::Unknown`'s `raw_fields` (f3 onward), using the
    /// same tokenizing/parsing primitives the built-in families use.
    fn decode(raw_fields: &[String]) -> Self {
        PullTimer {
            caller: parse_guid(&raw_fields[0]).expect("caller guid"),
            seconds: raw_fields[1].parse().expect("seconds"),
            label: unquote(&raw_fields[2]).to_string(),
            participants: parse_int_array(&raw_fields[3], 6).expect("participants"),
        }
    }

    /// Encode back to the same wire shape (f3 onward), using the crate's own
    /// rendering primitives so quoting/list-bracketing stay identical to the
    /// built-in families'.
    fn encode_fields(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.caller.render(),
            render_float(self.seconds, "seconds").unwrap(),
            render_quoted(&self.label, "label").unwrap(),
            render_int_array(&self.participants)
        )
    }
}

#[test]
fn a_custom_event_type_round_trips_through_unknown() {
    let instant = LogInstant { utc_ms: 0, seq: 1 };
    let original = PullTimer {
        caller: Guid::Player(7),
        seconds: 5.0,
        label: "Pull in 5".to_string(),
        participants: vec![1, 2, 3],
    };

    // The consumer assembles the full line by hand: this crate has no idea
    // what CUSTOM_PULL_TIMER means, so `encode_line` can't do it for them.
    let line = format!(
        "{}|{}|{}",
        render_instant(instant.utc_ms).unwrap(),
        PullTimer::TYPE,
        original.encode_fields()
    );

    let event =
        parse_line(instant.seq, &line).expect("a well-formed unrecognized line still parses");
    let (raw_type, raw_fields) = match &event.body {
        EventBody::Unknown {
            raw_type,
            raw_fields,
        } => (raw_type, raw_fields),
        other => panic!("expected Unknown, got {other:?}"),
    };
    assert_eq!(raw_type, PullTimer::TYPE);

    let decoded = PullTimer::decode(raw_fields);
    assert_eq!(decoded, original);

    // This crate's own `encode_line` still refuses it — the type is opaque here...
    assert!(matches!(
        encode_line(&event),
        Err(EncodeError::Unrepresentable { .. })
    ));

    // ...but the consumer, who does know what it means, can round-trip it losslessly.
    let re_encoded = format!(
        "{}|{}|{}",
        render_instant(event.instant.utc_ms).unwrap(),
        PullTimer::TYPE,
        decoded.encode_fields()
    );
    assert_eq!(re_encoded, line);
}
