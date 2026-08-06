import assert from "node:assert/strict";
import { test } from "node:test";

import { listCombatants } from "../dist/index.js";
import { fixtureText } from "./helpers.js";

test("combatant_info.log: finds the one combatant, gear[0] a null (unreadable) slot", async () => {
  const combatants = await listCombatants(fixtureText("combatant_info.log"));

  assert.equal(combatants.length, 1);
  const [combatant] = combatants;
  assert.equal(combatant.ulid, "01AAAAAAAAAAAAAAAAAAAAAAAA");
  assert.equal(combatant.name, "P1");
  assert.equal(combatant.isRecordingPlayer, true);
  assert.equal(combatant.snapshotCount, 1);
  assert.deepEqual(combatant.info.gear, [null]);
});

test("combatant_info_v8.log: finds both combatants with a fully-decoded gear piece each", async () => {
  const combatants = await listCombatants(fixtureText("combatant_info_v8.log"));

  assert.equal(combatants.length, 2);
  const [first, second] = combatants;

  assert.equal(first.ulid, "01AAAAAAAAAAAAAAAAAAAAAAAA");
  assert.equal(first.isRecordingPlayer, true);
  assert.equal(first.info.gear.length, 14);
  assert.deepEqual(first.info.gear[0], {
    itemId: 5213,
    itemLevel: 315,
    rarity: 5,
    temper: [8, 8],
    stats: [
      [1, 34],
      [3, 17],
      [23, 18],
      [3, 4],
      [3, 4],
      [26, 83],
    ],
    setBonusId: 686,
    abilityGrants: [[24, 2]],
    traits: [],
    gems: [],
    score: 572.8,
  });

  assert.equal(second.ulid, "01BBBBBBBBBBBBBBBBBBBBBBBB");
  assert.equal(second.isRecordingPlayer, false);
  assert.equal(second.info.gear.length, 14);
});

test("a log with no COMBATANT_INFO lines yields no combatants, without throwing", async () => {
  assert.deepEqual(await listCombatants(fixtureText("damage.log")), []);
});

test("malformed.log: unparseable lines are skipped rather than failing the scan", async () => {
  assert.deepEqual(await listCombatants(fixtureText("malformed.log")), []);
});
