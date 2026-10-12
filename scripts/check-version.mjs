// Checks that the version is set in one place (package.json) and mirrored exactly:
// src-tauri/Cargo.toml must match, tauri.conf.json must read it from package.json,
// and T0–T2 versions are 4.0.0-alpha.N (Master Prompt A11).
import { readFileSync } from 'node:fs';

const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8');
const conf = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargo)?.[1];

const errors = [];
if (!/^4\.0\.0-(alpha|beta)\.\d+$|^4\.\d+\.\d+$/.test(pkg.version)) errors.push(`unexpected version format: ${pkg.version}`);
if (cargoVersion !== pkg.version) errors.push(`Cargo.toml has ${cargoVersion}, package.json has ${pkg.version}`);
if (conf.version !== '../package.json') errors.push(`tauri.conf.json version must be "../package.json", is ${conf.version}`);

if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.warn(`version ${pkg.version} OK`);
