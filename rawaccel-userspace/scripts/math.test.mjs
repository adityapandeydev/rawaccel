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
