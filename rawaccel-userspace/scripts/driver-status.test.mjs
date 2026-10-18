import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const source = readFileSync(new URL('../src/driverStatus.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { createDriverStatusMonitor, parseDriverStatus, versionText, driverStatusView, applyBlockReason } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString('base64')}`);
const connectedFixture = JSON.parse(readFileSync(new URL('../src-tauri/tests/fixtures/driver-status/stock-connected.json', import.meta.url), 'utf8'));
const connected = () => structuredClone(connectedFixture);
const unavailable = () => ({ ...connected(), state: 'unavailable', version: null, compatibility: 'unknown', diagnostic: { stage: 'open', win32_code: 2, hresult: '0x80070002', message: 'Device not found' } });
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function environment() {
  let time = 0, visible = true;
  const events = { focus: new Set(), visibility: new Set(), interval: new Set() };
  const bind = event => fn => { events[event].add(fn); return () => events[event].delete(fn); };
  return {
    api: { now: () => time, visible: () => visible, onFocus: bind('focus'), onVisibility: bind('visibility'), every: (fn, ms) => { assert.equal(ms, 30000); return bind('interval')(fn); } },
    advance: ms => { time += ms; },
    visible: value => { visible = value; },
    emit: event => { for (const fn of events[event]) fn(); },
    listenerCount: () => Object.values(events).reduce((sum, set) => sum + set.size, 0),
  };
}

test('parse the actual connected version separately from custom ABI compatibility', () => {
  const report = parseDriverStatus(connected());
  assert.equal(versionText(report.version), '1.7.0');
  assert.equal(report.compatibility, 'supported');
  const other = { ...connected(), version: { major: 1, minor: 7, patch: 5 }, compatibility: 'unsupported' };
  assert.equal(versionText(parseDriverStatus(other).version), '1.7.5');
  assert.deepEqual(parseDriverStatus(unavailable()), unavailable());
});

test('reject malformed or inconsistent IPC payloads', () => {
  for (const value of [null, {}, { ...connected(), version: null }, { ...connected(), compatibility: 'unknown' },
    { ...connected(), version: { major: 1, minor: -1, patch: 0 } },
    { ...connected(), compatibility: 'unsupported' },
    { ...unavailable(), diagnostic: null }, { ...unavailable(), diagnostic: { ...unavailable().diagnostic, win32_code: -1 } }]) {
    assert.throws(() => parseDriverStatus(value), /invalid driver-status response/);
  }
});

test('startup and simultaneous refreshes share one pending query', async () => {
  const env = environment(), pending = deferred();
  let calls = 0;
  const monitor = createDriverStatusMonitor(() => { calls++; return pending.promise; }, env.api);
  assert.equal(monitor.getSnapshot().checking, true);
  monitor.start();
  const first = monitor.refresh(), second = monitor.refresh();
  assert.equal(first, second);
  await flush(); assert.equal(calls, 1);
  pending.resolve(connected()); await first;
  assert.equal(monitor.getSnapshot().report.state, 'connected');
  assert.equal(monitor.getSnapshot().checking, false);
  monitor.stop();
});

test('automatic refresh is throttled and pauses while hidden', async () => {
  const env = environment(); let calls = 0;
  const monitor = createDriverStatusMonitor(async () => { calls++; return connected(); }, env.api);
  monitor.start(); await flush();
  env.emit('focus'); await flush(); assert.equal(calls, 1);
  env.advance(5000); env.emit('focus'); await flush(); assert.equal(calls, 2);
  env.visible(false); env.advance(30000); env.emit('interval'); env.emit('visibility'); await flush(); assert.equal(calls, 2);
  env.visible(true); env.emit('visibility'); await flush(); assert.equal(calls, 3);
  await monitor.refresh(); assert.equal(calls, 4); // Manual refresh bypasses cooldown.
  monitor.stop(); assert.equal(env.listenerCount(), 0);
});

test('an IPC failure clears stale Connected state and a later refresh recovers', async () => {
  const env = environment(); let fail = false;
  const monitor = createDriverStatusMonitor(async () => { if (fail) throw new Error('IPC disconnected'); return connected(); }, env.api);
  monitor.start(); await flush(); fail = true;
  await monitor.refresh();
  assert.equal(monitor.getSnapshot().report, null);
  assert.equal(monitor.getSnapshot().ipcError, 'IPC disconnected');
  fail = false; await monitor.refresh();
  assert.equal(monitor.getSnapshot().ipcError, null);
  assert.equal(monitor.getSnapshot().report.state, 'connected');
  monitor.stop();
});

test('stopping suppresses late results and removes every listener and timer', async () => {
  const env = environment(), pending = deferred(); let updates = 0;
  const monitor = createDriverStatusMonitor(() => pending.promise, env.api);
  const unsubscribe = monitor.subscribe(() => { updates++; });
  monitor.start(); await flush(); const before = updates;
  monitor.stop(); pending.resolve(connected()); await flush();
  assert.equal(updates, before); assert.equal(env.listenerCount(), 0);
  unsubscribe();
});

test('StrictMode restart queues a new query without overlapping the old one', async () => {
  const env = environment(), old = deferred(), fresh = deferred(); let calls = 0;
  const monitor = createDriverStatusMonitor(() => (++calls === 1 ? old.promise : fresh.promise), env.api);
  monitor.start(); await flush(); monitor.stop(); monitor.start(); await flush();
  assert.equal(calls, 1); old.resolve(unavailable()); await flush();
  assert.equal(calls, 2); assert.equal(monitor.getSnapshot().report, null);
  fresh.resolve(connected()); await flush();
  assert.equal(monitor.getSnapshot().report.state, 'connected');
  assert.equal(env.listenerCount(), 3); monitor.stop();
});

test('Apply queues a fresh check if an older status request is pending', async () => {
  const env = environment(), old = deferred(); let calls = 0;
  const monitor = createDriverStatusMonitor(() => ++calls === 1 ? old.promise : Promise.resolve(unavailable()), env.api);
  monitor.start(); await flush(); void monitor.afterApply(); old.resolve(connected()); await flush();
  assert.equal(calls, 2);
  assert.equal(monitor.getSnapshot().report.state, 'unavailable'); monitor.stop();
});

test('all failure states have distinct feedback and never claim a version is installed', () => {
  const labels = new Set();
  for (const state of ['unavailable', 'access_denied', 'busy', 'open_failed', 'query_failed', 'malformed_response']) {
    const snapshot = { report: { ...unavailable(), state }, checking: false, ipcError: null, checkedAt: 1 };
    const view = driverStatusView(snapshot); labels.add(view.label);
    assert.ok(view.description.length > 20);
    assert.ok(applyBlockReason(snapshot, []));
    assert.ok(!view.label.includes('1.7.0'));
  }
  assert.equal(labels.size, 6);
});

test('Tiered stays preview-only regardless of Gain or Combined mode in stock builds', () => {
  const snapshot = { report: connected(), checking: false, ipcError: null, checkedAt: 1 };
  const profile = { name: 'Preview', accel_x: { mode: 'classic' }, accel_y: { mode: 'noaccel' } };
  assert.equal(applyBlockReason(snapshot, [profile]), null);
  for (const axis of ['accel_x', 'accel_y']) {
    const preview = structuredClone(profile); preview[axis].mode = 'tiered';
    assert.match(applyBlockReason(snapshot, [preview]), /preview only/);
  }
  const custom = { ...snapshot, report: { ...connected(), tiered_apply_enabled: true, driver_contract: 'rawaccel-tiered-v1', compatibility: 'unverified' } };
  assert.equal(applyBlockReason(custom, [{ ...profile, accel_x: { mode: 'tiered' } }]), null);
  assert.match(driverStatusView(custom).description, /does not verify/);
});

test('checking, IPC errors and unsupported versions block Apply without a stale green indicator', () => {
  const base = { report: connected(), checking: false, ipcError: null, checkedAt: 1 };
  for (const snapshot of [{ ...base, checking: true }, { ...base, report: null, ipcError: 'IPC failed' },
    { ...base, report: { ...connected(), version: { major: 1, minor: 8, patch: 0 }, compatibility: 'unsupported' } }]) {
    assert.ok(applyBlockReason(snapshot, []));
    assert.notEqual(driverStatusView(snapshot).tone, 'success');
  }
});
