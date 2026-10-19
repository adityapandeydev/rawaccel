import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { parseWindowState, restoreWindowState } = await loadTs('windowState');
const primary = { x: 0, y: 0, width: 1920, height: 1040, scale: 1 };
const state = { x: 200, y: 100, width: 1280, height: 860, maximized: false };
test('window state rejects missing, corrupt or invalid storage', () => {
  for (const value of [null, '{', '{}', JSON.stringify({...state, width:-1}), JSON.stringify({...state,x:1e9}),JSON.stringify({...state,maximized:0})]) assert.equal(parseWindowState(value),null);
  assert.deepEqual(parseWindowState(JSON.stringify(state)),state);
});
test('normal bounds and maximized preference survive a session', () => {
  assert.deepEqual(restoreWindowState(state,[primary]),state);
  assert.deepEqual(restoreWindowState({...state,maximized:true},[primary]),{...state,maximized:true});
});
test('removed monitors and oversized bounds recover a reachable title bar', () => {
  const restored=restoreWindowState({...state,x:3000,y:-200,width:5000,height:3000},[primary]);
  assert.deepEqual(restored,{x:0,y:0,width:1920,height:1040,maximized:false});
  assert.equal(restoreWindowState(state,[]),null);
});
test('negative monitor coordinates and minimum logical sizes respect DPI', () => {
  const monitor={x:-2560,y:0,width:2560,height:1440,scale:1.5};
  const restored=restoreWindowState({...state,x:-2500,width:400,height:200},[primary,monitor]);
  assert.equal(restored.x,-2500);
  assert.equal(restored.width,1536);
  assert.equal(restored.height,1152);
});
