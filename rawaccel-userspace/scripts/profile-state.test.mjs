import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { renameProfileState, deleteProfileState } = await loadTs('profileState');
const profiles = [{ name: 'A' }, { name: 'B' }, { name: 'C' }];
const devices = [{ id: 'a', profile_id: 'A' }, { id: 'b', profile_id: 'B' }, { id: 'none', profile_id: null }];

test('rename preserves mappings through an empty intermediate editor value', () => {
  const empty = renameProfileState(profiles, devices, 1, '');
  const renamed = renameProfileState(empty.profiles, empty.devices, 1, 'New');
  assert.equal(renamed.devices[1].profile_id, 'New');
  assert.deepEqual(renamed.devices[0], devices[0]);
  assert.equal(renamed.devices[2].profile_id, null);
});

test('duplicate rename cannot merge two profiles device assignments', () => {
  assert.throws(() => renameProfileState(profiles, devices, 1, ' a '), /already exists/);
  assert.equal(devices[1].profile_id, 'B');
});

test('delete preserves active identity, clears only deleted mappings and keeps a last profile', () => {
  const before = deleteProfileState(profiles, devices, 2, 0);
  assert.equal(before.profiles[before.activeIndex].name, 'C');
  assert.equal(before.devices[0].profile_id, null);
  assert.equal(before.devices[1].profile_id, 'B');
  const active = deleteProfileState(profiles, devices, 1, 1);
  assert.equal(active.profiles[active.activeIndex].name, 'A');
  assert.equal(active.devices[1].profile_id, null);
  assert.throws(() => deleteProfileState([profiles[0]], devices, 0, 0), /last profile/);
});
