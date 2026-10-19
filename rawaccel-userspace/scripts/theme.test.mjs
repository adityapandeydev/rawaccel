import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { loadTs } from './load-ts.mjs';
const { parseTheme, resolveTheme } = await loadTs('theme');
test('theme preference recovers bad storage and follows system only when selected', () => {
  for (const value of [null, '', 'pink']) assert.equal(parseTheme(value), 'system');
  assert.equal(resolveTheme('system', true), 'dark');
  assert.equal(resolveTheme('system', false), 'light');
  assert.equal(resolveTheme('dark', false), 'dark');
  assert.equal(resolveTheme('light', true), 'light');
});
function luminance(hex) {
  const rgb=hex.match(/[a-f0-9]{2}/gi).map(v=>parseInt(v,16)/255).map(v=>v<=0.04045?v/12.92:((v+0.055)/1.055)**2.4);
  return rgb[0]*0.2126+rgb[1]*0.7152+rgb[2]*0.0722;
}
const palette=JSON.parse(readFileSync(new URL('../src/palette.json',import.meta.url)));
for(const [theme,background] of [['dark','#181d23'],['light','#ffffff']]) {
  test(`all five ${theme} graph roles are distinct and exceed 3:1 contrast`,()=>{
    assert.equal(new Set(Object.values(palette[theme])).size,5);
    for(const color of Object.values(palette[theme])) {
      const values=[luminance(color),luminance(background)].sort((a,b)=>a-b);
      assert.ok((values[1]+0.05)/(values[0]+0.05)>=3,`${color} on ${background}`);
    }
  });
}
