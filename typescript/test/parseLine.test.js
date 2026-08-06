import assert from "node:assert/strict";
import { test } from "node:test";

import { parseLine } from "../dist/index.js";
import { fixtureLines } from "./helpers.js";

test("damage.log: decodes a critical-strike ability-damage line", async () => {
  const [{ seq, line }] = fixtureLines("damage.log");
  const event = await parseLine(seq, line);

  assert.equal(event.instant.seq, seq);
  assert.equal(typeof event.instant.utcMs, "number");
  assert.equal(event.body.type, "DamageHeal");
  assert.equal(event.body.kind, "AbilityDamage");
  assert.deepEqual(event.body.source, { type: "Player", id: 2000000001 });
  assert.deepEqual(event.body.target, {
    type: "Npc",
    spawn: 3049784064,
    template: 42,
  });
  assert.equal(event.body.abilityId, 2669);
  assert.equal(event.body.school, "Magical");
  assert.equal(event.body.result, "CriticalStrike");
  assert.equal(event.body.applied, 112);
  assert.equal(event.body.sourceCurHp, 43441);
  assert.equal(event.body.targetCurHp, 326373);
});

test("spell_cast.log: decodes the cast/channel phase family", async () => {
  const [activated, castStart, castFail] = fixtureLines("spell_cast.log");

  const activatedEvent = await parseLine(activated.seq, activated.line);
  assert.equal(activatedEvent.body.type, "Cast");
  assert.deepEqual(activatedEvent.body.phase, { type: "Activated" });
  assert.deepEqual(activatedEvent.body.caster, { type: "Player", id: 4000000001 });
  assert.equal(activatedEvent.body.abilityId, 2050);
  assert.equal(activatedEvent.body.hasTarget, false);
  assert.deepEqual(activatedEvent.body.target, { type: "Unrecognized" });

  const castStartEvent = await parseLine(castStart.seq, castStart.line);
  assert.equal(castStartEvent.body.type, "Cast");
  assert.equal(castStartEvent.body.phase.type, "CastStart");
  assert.equal(castStartEvent.body.phase.castSeconds, 1.5);

  const castFailEvent = await parseLine(castFail.seq, castFail.line);
  assert.equal(castFailEvent.body.type, "Cast");
  assert.deepEqual(castFailEvent.body.phase, {
    type: "CastFail",
    reason: "AbilityFailed.CastCancelled",
  });
  assert.deepEqual(castFailEvent.body.target, {
    type: "Npc",
    spawn: 3206022608,
    template: 41,
  });
});

test("effect_aura.log: decodes applied/removed/refreshed with the right polarity and refresher", async () => {
  const [applied, removed, refreshed] = fixtureLines("effect_aura.log");

  const appliedEvent = await parseLine(applied.seq, applied.line);
  assert.equal(appliedEvent.body.type, "Effect");
  assert.equal(appliedEvent.body.phase, "Applied");
  assert.equal(appliedEvent.body.polarity, "Debuff");
  assert.equal(appliedEvent.body.effectId, 101);
  assert.equal(appliedEvent.body.durationSeconds, 8);
  assert.equal(appliedEvent.body.refresher, null);

  const removedEvent = await parseLine(removed.seq, removed.line);
  assert.equal(removedEvent.body.type, "Effect");
  assert.equal(removedEvent.body.phase, "Removed");
  assert.equal(removedEvent.body.polarity, "Buff");
  assert.equal(removedEvent.body.refresher, null);

  const refreshedEvent = await parseLine(refreshed.seq, refreshed.line);
  assert.equal(refreshedEvent.body.type, "Effect");
  assert.equal(refreshedEvent.body.phase, "Refreshed");
  assert.deepEqual(refreshedEvent.body.refresher, {
    type: "Npc",
    spawn: 3049784064,
    template: 42,
  });
});

test("combatant_info.log: decodes identity, stat sheet, and one gear piece", async () => {
  const [{ seq, line }] = fixtureLines("combatant_info.log");
  const event = await parseLine(seq, line);

  assert.equal(event.body.type, "CombatantInfo");
  assert.equal(event.body.ulid, "01AAAAAAAAAAAAAAAAAAAAAAAA");
  assert.deepEqual(event.body.player, { type: "Player", id: 1000000001 });
  assert.equal(event.body.isRecordingPlayer, true);
  assert.equal(event.body.heroId, 10);
  assert.equal(event.body.itemLevel, 318.2);
  assert.deepEqual(event.body.talents, [75, 81, 441]);
  // The fixture's one gear-slot tuple is one field short of the 13 the decoder requires (a
  // deliberately-truncated/unrecognized grammar), so it decodes to a `null` slot rather than
  // being dropped — see `GearPiece`'s doc comment on why slot position must be preserved.
  assert.deepEqual(event.body.gear, [null]);

  assert.deepEqual(event.body.traitRanks, [
    [1, 10],
    [52, 2],
  ]);
  assert.deepEqual(event.body.neckTraits, [
    { traitId: 60, selected: true },
    { traitId: 61, selected: false },
  ]);
});

test("timezone_glitch.log: a one-minute-earlier offset still yields the same UTC instant", async () => {
  const [first, second] = fixtureLines("timezone_glitch.log");

  const firstEvent = await parseLine(first.seq, first.line);
  const secondEvent = await parseLine(second.seq, second.line);

  assert.equal(firstEvent.instant.utcMs, secondEvent.instant.utcMs);
});

test("malformed.log: every line rejects with an Error, never resolves", async () => {
  for (const { seq, line } of fixtureLines("malformed.log")) {
    await assert.rejects(() => parseLine(seq, line), Error);
  }
});

test("an unrecognized event type decodes as Unknown rather than throwing", async () => {
  const event = await parseLine(1, '2026-07-22T10:29:06.540+02:00|SOME_FUTURE_EVENT|1|2|3');
  assert.deepEqual(event.body, { type: "Unknown", rawType: "SOME_FUTURE_EVENT" });
});
