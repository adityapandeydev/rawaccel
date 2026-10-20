// Check the actual generated installer payload, including local scratch-file leaks.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const config = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri/tauri.conf.json')));
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json')));
const cargoVersion = fs.readFileSync(path.join(root, 'src-tauri/Cargo.toml'), 'utf8').match(/^version = "([^"]+)"/m)[1];
assert.equal(config.version, pkg.version, 'Frontend and installer versions must agree');
assert.equal(config.version, cargoVersion, 'Native and installer versions must agree');
const script = fs.readFileSync(path.join(root, 'src-tauri/target/release/nsis/x64/installer.nsi'), 'utf8');
const allowed = new Set([
  '${MAINBINARYSRCPATH}', 'LICENSE.txt', 'RELEASE.txt',
  '$TEMP\\MicrosoftEdgeWebview2Setup.exe', '$TEMP\\MicrosoftEdgeWebView2RuntimeInstaller.exe',
]);
const payloads = script.split(/\r?\n/).filter(line => /^\s*File\s/.test(line)).map(line => {
  const target = line.match(/\/oname=([^"]+)/)?.[1] ?? line.match(/File\s+"([^"]+)"/)?.[1];
  assert.ok(allowed.has(target), `Unexpected installer payload: ${line.trim()}`);
  return target;
});
assert.ok(payloads.includes('${MAINBINARYSRCPATH}'), 'Application executable is missing');
assert.ok(payloads.includes('LICENSE.txt'), 'License is missing');
assert.ok(payloads.includes('RELEASE.txt'), 'User guide is missing');
const installer = path.join(root, `src-tauri/target/release/bundle/nsis/RawAccel_${config.version}_x64-setup.exe`);
assert.ok(fs.statSync(installer).size > 0, 'Installer artifact is missing');
console.log('Installer payload verified: one application, guide, license and WebView2 bootstrapper; versions agree.');
