import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { documentStatus } = await loadTs('documentStatus');
test('loading a saved file never claims the driver is applied', () => {
  assert.deepEqual(documentStatus('a','a','loaded',null), {edited:false,disk:'Loaded from disk',driver:'Driver state not verified'});
});
test('partial disk failure preserves the applied state and unsaved edits', () => {
  assert.deepEqual(documentStatus('b','a','loaded','b'), {edited:true,disk:'Unsaved changes',driver:'Applied this session'});
});
test('edits made during Apply are not marked saved or applied by its late result', () => {
  assert.deepEqual(documentStatus('c','b','saved','b'), {edited:true,disk:'Unsaved changes',driver:'Changed since last Apply'});
  assert.deepEqual(documentStatus('b','b','saved','b'), {edited:false,disk:'Saved to disk',driver:'Applied this session'});
});
test('reverting disk edits does not claim to revert the driver', () => {
  const status=documentStatus('a','a','loaded','b');
  assert.equal(status.edited,false);
  assert.equal(status.driver,'Changed since last Apply');
});
