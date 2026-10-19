import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { createApplyGate } = await loadTs('applyGate');

test('overlapping Apply controls send only the first snapshot; subsequent Apply can run', async () => {
  const run = createApplyGate();
  let finish;
  let writes = 0;
  const first = run(() => { writes++; return new Promise(resolve => { finish = resolve; }); });
  assert.equal(await run(async () => { writes++; return true; }), false);
  assert.equal(writes, 1);
  finish(true);
  assert.equal(await first, true);
  assert.equal(await run(async () => { writes++; return true; }), true);
  assert.equal(writes, 2);
});

test('failed and applied-but-unsaved outcomes both permit retry', async () => {
  const run = createApplyGate();
  await assert.rejects(run(async () => { throw new Error('driver unavailable'); }));
  assert.equal(await run(async () => false), false);
  assert.equal(await run(async () => true), true);
});
