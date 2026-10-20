import assert from 'node:assert/strict';
import test from 'node:test';
import { loadTs } from './load-ts.mjs';
const { parsePreferences, notificationChannel } = await loadTs('preferences');

test('preferences recover missing/corrupt storage without enabling unsupported languages', () => {
  for (const raw of [null, '', 'broken', 'null', '[]', '{"showToastNotifications":"false"}']) {
    assert.deepEqual(parsePreferences(raw), { showToastNotifications: true, showConfirmModals: true });
  }
  assert.deepEqual(parsePreferences('{"showToastNotifications":false,"showConfirmModals":false,"language":"ja-JP"}'),
    { showToastNotifications: false, showConfirmModals: false });
});

test('informational toasts honor the preference while errors remain visible', () => {
  assert.equal(notificationChannel('info', true), 'toast');
  assert.equal(notificationChannel('info', false), 'none');
  assert.equal(notificationChannel('error', false), 'dialog');
  assert.equal(notificationChannel(undefined, false), 'dialog');
});
