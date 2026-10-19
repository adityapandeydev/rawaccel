import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { loadTs } from './load-ts.mjs';
const { splitSettingsDocument, serializeSettingsDocument } = await loadTs('settingsDocument');

test('load/export/Apply preserve complete profiles, devices and unknown document metadata', () => {
  const original = JSON.parse(readFileSync(new URL('../src-tauri/tests/fixtures/settings/modern-complete.json', import.meta.url)));
  original.future_editor = { nested: [0, false, null, 'retain'] };
  const { profiles, devices, metadata } = splitSettingsDocument(original);
  assert.equal(metadata.profiles, undefined);
  assert.equal(metadata.devices, undefined);
  assert.deepEqual(JSON.parse(serializeSettingsDocument(metadata, profiles, devices)), original);
  assert.deepEqual(JSON.parse(serializeSettingsDocument(metadata, profiles, devices, true)), original);
  assert.deepEqual(original.future_editor, { nested: [0, false, null, 'retain'] });
});

test('editable fields replace any stale reserved metadata keys', () => {
  const encoded = serializeSettingsDocument({ profiles: ['stale'], devices: ['stale'], schema_version: 1 }, [{name:'current'}], []);
  assert.deepEqual(JSON.parse(encoded), {schema_version:1, profiles:[{name:'current'}], devices:[]});
});
