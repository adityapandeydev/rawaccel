import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { defaultDeviceConfig, patchDevice, resolveDeviceSelection } = await loadTs('deviceState');

test('late devices, disconnect and reconnect resolve to a connected selection', () => {
  assert.equal(resolveDeviceSelection(null, []), null);
  assert.equal(resolveDeviceSelection(null, [{ id: 'a' }]), 'a');
  assert.equal(resolveDeviceSelection('b', [{ id: 'a' }, { id: 'b' }]), 'b');
  assert.equal(resolveDeviceSelection('b', [{ id: 'a' }]), 'a');
  assert.equal(resolveDeviceSelection('b', []), null);
});

test('first edit creates unassigned config; later edits retain mappings and disconnected entries', () => {
  const before = [{ ...defaultDeviceConfig('offline'), profile_id: 'Other', dpi: 1600 }];
  const edited = patchDevice(before, 'new', { dpi: 800 });
  assert.equal(edited[1].profile_id, null);
  assert.equal(edited[1].dpi, 800);
  assert.deepEqual(edited[0], before[0]);
  assert.equal(before.length, 1);
  const next = patchDevice(edited, 'new', { poll_time_lock: true });
  assert.equal(next.length, 2);
  assert.equal(next[1].dpi, 800);
  assert.equal(next[1].poll_time_lock, true);
});
