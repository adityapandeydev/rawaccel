import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const defaults = JSON.parse(readFileSync(new URL('../src/settings-defaults.json', import.meta.url), 'utf8'));
// Exercise the real TypeScript factories in Node without introducing a test
// runner dependency or changing browser module resolution for JSON imports.
const source = readFileSync(new URL('../src/types.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const importPattern = /import settingsDefaults from ["']\.\/settings-defaults\.json["'];?/;
assert.match(compiled, importPattern);
const executable = compiled.replace(importPattern, () => `const settingsDefaults = ${JSON.stringify(defaults)};`);
const factories = await import(`data:text/javascript;base64,${Buffer.from(executable).toString('base64')}`);

test('frontend factories use the shared backend default template', () => {
  assert.deepEqual(factories.defaultAccelArgs(), defaults.accel_args);
  assert.deepEqual(factories.defaultSpeedArgs(), defaults.speed_args);
  assert.equal(factories.SETTINGS_SCHEMA_VERSION, defaults.schema_version);
});

test('editing one axis or profile cannot mutate another profile or the template', () => {
  const first = factories.defaultProfile('First');
  const second = factories.defaultProfile('Second');
  assert.notEqual(first.id, second.id);
  assert.equal(first.name, 'First');
  first.accel_x.cap.x = 999;
  first.accel_x.lookup_table.push({ x: 1, y: 2 });
  first.domain_weights.x = 7;
  assert.equal(first.accel_y.cap.x, 15);
  assert.equal(second.accel_x.cap.x, 15);
  assert.deepEqual(second.accel_x.lookup_table, []);
  assert.equal(second.domain_weights.x, 1);
  assert.equal(defaults.accel_args.cap.x, 15);
});
