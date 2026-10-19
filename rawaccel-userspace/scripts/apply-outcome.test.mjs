import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { describeApplyOutcome } = await loadTs('applyOutcome');
const saved = { applied:true, saved:true, warnings:[], saved_paths:['portable','mirror'], save_error:null };
test('full success needs no warning and permits success animation', () => {
  assert.deepEqual(describeApplyOutcome(saved), {succeeded:true});
});
test('partial save reports durable destinations and failed locations', () => {
  const result = describeApplyOutcome({...saved,warnings:['mirror denied'],saved_paths:['portable']});
  assert.equal(result.succeeded,true);
  assert.equal(result.notification.title,'Applied and Saved With Warnings');
  assert.match(result.notification.message,/portable/);
  assert.match(result.notification.message,/mirror denied/);
});
test('applied-but-unsaved outcome never flashes full success', () => {
  const result=describeApplyOutcome({...saved,saved:false,save_error:'disk full'});
  assert.equal(result.succeeded,false);
  assert.match(result.notification.message,/disk full/);
  assert.match(describeApplyOutcome({...saved,saved:false}).notification.message,/No settings location/);
});
