import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const readJson = path => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const defaults = readJson('../src/settings-defaults.json').accel_args;
const source = readFileSync(new URL('../src/math.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { createEvaluator, generateGraphData } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString('base64')}`);

test('Synchronous Gain is independent of evaluation order and previous queries', () => {
  const args = { ...structuredClone(defaults), mode: 'synchronous', gain: true };
  const reused = createEvaluator(args);
  for (const x of [100, 2.123, 0.125, 512, 0, 10, 1024, 1, 100, 0.01]) {
    assert.equal(reused(x), createEvaluator(args)(x), `speed ${x}`);
  }
});

test('Synchronous Gain uses the first velocity sample below the sampled range', () => {
  const args = { ...structuredClone(defaults), mode: 'synchronous', gain: true,
    motivity: 2, sync_speed: 10, gamma: 1, smooth: 0 };
  const evaluate = createEvaluator(args);
  for (const x of [0, 0.01, 0.125, 0.5]) assert.equal(evaluate(x), 0.5);
});

test('all three generated graph quantities stay available for Synchronous Gain', () => {
  const data = generateGraphData({ ...structuredClone(defaults), mode: 'synchronous' });
  assert.ok(data.length > 100);
  assert.ok(data.every(point => Object.values(point).every(Number.isFinite)));
});

const reference = readJson('../src-tauri/tests/fixtures/math/reference.json');
let maximumAbsoluteError = 0;
let maximumRelativeError = 0;
const floatBuffer = new DataView(new ArrayBuffer(4));
function floatBits(value) {
  floatBuffer.setFloat32(0, value);
  return floatBuffer.getUint32(0);
}
for (const fixture of reference.cases) {
  test(`C++ parity: ${fixture.name}`, () => {
    assert.equal(fixture.xs.length, fixture.expected.length);
    assert.equal(fixture.table_bits.length, fixture.args.mode === 'synchronous' && fixture.args.gain ? 97 : 0);
    const evaluate = createEvaluator(fixture.args);
    for (let i = 0; i < fixture.table_bits.length; i++) {
      const e = Math.floor(i / 8) - 3;
      const x = (i % 8 + 8) * 2 ** e / 8;
      assert.equal(floatBits(evaluate(x) * x), fixture.table_bits[i], `packed sample ${i}`);
    }
    for (const index of fixture.xs.map((_, i) => i).reverse()) {
      const x = fixture.xs[index];
      const expected = fixture.expected[index];
      const actual = evaluate(x);
      if (typeof expected === 'string') {
        assert.equal(String(actual), expected, `speed ${x}`);
      } else {
        const error = Math.abs(actual - expected);
        maximumAbsoluteError = Math.max(maximumAbsoluteError, error);
        if (expected !== 0) maximumRelativeError = Math.max(maximumRelativeError, error / Math.abs(expected));
        const tolerance = reference.absolute_tolerance + reference.relative_tolerance * Math.abs(expected);
        assert.ok(Number.isFinite(actual) && Math.abs(actual - expected) <= tolerance,
          `speed ${x}: actual ${actual}, C++ ${expected}, tolerance ${tolerance}`);
      }
    }
  });
}

test('Tiered Natural preserves velocity and slope continuity at the second transition', () => {
  for (const gain of [false, true]) {
    const args = { ...structuredClone(defaults), mode: 'tiered', t_type: 'natural', gain,
      tiered_input_offset1: 5, tiered_input_offset2: 30 };
    const evaluate = createEvaluator(args);
    const x = args.tiered_input_offset2;
    const epsilon = 1e-5;
    const v = speed => speed * evaluate(speed);
    assert.ok(Math.abs(evaluate(x - epsilon) - evaluate(x + epsilon)) < 1e-5);
    assert.ok(Math.abs((v(x) - v(x - epsilon)) / epsilon - (v(x + epsilon) - v(x)) / epsilon) < 1e-5);
  }
});

test('report measured C++ preview comparison errors', context => {
  context.diagnostic(`${reference.cases.length} configurations: max absolute error ${maximumAbsoluteError}, max relative error ${maximumRelativeError}`);
});
