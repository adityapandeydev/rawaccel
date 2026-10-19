import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { trackerInputs } = await loadTs('tracker');
test('processing mode selects tracker axes even for identical curves', () => {
  const speed = { speed_x: 3, speed_y: 4, speed_whole: 5 };
  assert.deepEqual(trackerInputs(true, speed), [{ axis: 'x', speed: 5 }]);
  assert.deepEqual(trackerInputs(false, speed), [{ axis: 'x', speed: 3 }, { axis: 'y', speed: 4 }]);
  assert.equal(trackerInputs(false, { ...speed, speed_x: NaN }).length, 1);
});
