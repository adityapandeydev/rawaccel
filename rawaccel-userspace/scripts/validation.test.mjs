import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const readJson = path => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const defaults = readJson('../src/settings-defaults.json');
const cases = readJson('../src-tauri/tests/fixtures/settings/validation-cases.json');
const source = readFileSync(new URL('../src/validation.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { validateSettings } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString('base64')}`);

function document() {
  return {
    profiles: [{ ...structuredClone(defaults.profile), id: '',
      accel_x: structuredClone(defaults.accel_args),
      accel_y: structuredClone(defaults.accel_args),
      speed_processor_args: structuredClone(defaults.speed_args) }],
    devices: [], default_device_config: null,
  };
}

for (const fixture of cases) {
  test(fixture.name, () => {
    const value = document();
    for (const patch of fixture.patches) {
      const parts = patch.path.slice(1).split('/');
      const key = parts.pop();
      const parent = parts.reduce((object, part) => object[part], value);
      parent[key] = structuredClone(patch.value);
    }
    const error = validateSettings(value.profiles, value.devices, value.default_device_config);
    assert.equal(error === null, fixture.valid, error ?? 'Expected rejection');
  });
}

test('reject nonfinite input before JSON.stringify can turn it into null', () => {
  for (const number of [NaN, Infinity, -Infinity]) {
    const value = document();
    value.profiles[0].accel_y.gamma = number;
    assert.match(validateSettings(value.profiles, []), /finite/);
    value.profiles[0].accel_y.gamma = 1;
    value.profiles[0].output_dpi = number;
    assert.match(validateSettings(value.profiles, []), /finite/);
    value.profiles[0].output_dpi = 1000;
    assert.match(validateSettings(value.profiles, [{ id: 'mouse', clamp_max: number }]), /finite/);
  }
});

test('profile name uniqueness and exact wire mapping stay distinct', () => {
  const value = document();
  const duplicate = structuredClone(value.profiles[0]);
  duplicate.name = ` ${duplicate.name.toUpperCase()} `;
  assert.match(validateSettings([...value.profiles, duplicate], []), /Duplicate profile/);
  assert.match(validateSettings(value.profiles, [{ id: 'mouse', profile_id: 'default profile' }]), /non-existent/);
});
