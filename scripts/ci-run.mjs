// Runs one CI step and, when it fails, reports the failures as GitHub annotations
// (`::error file=…,line=…,title=…::message`), which can be read through api.github.com
// (check-runs → annotations) without downloading the job log (A11 rev. n: the CI loop).
//
// Usage (from the repository root, or with --cwd):
//   node scripts/ci-run.mjs "<step label>" [--cwd src-tauri] -- <command and arguments>
//
// The command's output is passed through live. On failure, known error formats become one
// annotation each (at most 10 per step, GitHub's limit): TypeScript / svelte-check, ESLint,
// rustc / clippy, cargo fmt, cargo test panics (with the assertion's left / right), and Vite /
// generic "Error:" lines. When nothing is recognised, one annotation carries the last lines.
// Vitest and Playwright report their own failures (github-actions / github reporters).
import { spawn } from 'node:child_process';
import path from 'node:path';

const args = process.argv.slice(2);
const label = args.shift() ?? 'step';
let cwd = process.cwd();
if (args[0] === '--cwd') {
  args.shift();
  cwd = path.resolve(args.shift() ?? '.');
}
if (args[0] === '--') args.shift();
const command = args.join(' ');
const root = process.cwd();
const MAX = 10;
const MAX_MESSAGE = 3500;
// Colour codes in tool output (ESC [ … m).
const ANSI = new RegExp(`${String.fromCharCode(27)}\\[[0-9;]*m`, 'g');
// Workflow commands must go to standard output.
const out = (line) => process.stdout.write(`${line}\n`);

const child = spawn(command, { cwd, shell: true, env: { ...process.env, FORCE_COLOR: '0', CARGO_TERM_COLOR: 'never' } });
let output = '';
child.stdout.on('data', (d) => {
  process.stdout.write(d);
  output += d.toString();
});
child.stderr.on('data', (d) => {
  process.stderr.write(d);
  output += d.toString();
});
child.on('close', (code) => {
  if (code !== 0) annotate(output.replace(ANSI, ''), code ?? 1);
  process.exit(code ?? 1);
});

/** A path GitHub can attach to: relative to the repository root, forward slashes. */
function rel(file) {
  const abs = path.isAbsolute(file) ? file : path.resolve(cwd, file);
  return path.relative(root, abs).split(path.sep).join('/');
}

const esc = (s) => s.replace(/%/g, '%25').replace(/\r/g, '%0D').replace(/\n/g, '%0A');
const escProp = (s) => esc(s).replace(/:/g, '%3A').replace(/,/g, '%2C');

function emit(a) {
  const props = [a.file && `file=${escProp(a.file)}`, a.line && `line=${a.line}`, a.col && `col=${a.col}`, `title=${escProp(a.title)}`]
    .filter(Boolean)
    .join(',');
  const msg = a.message.length > MAX_MESSAGE ? `${a.message.slice(0, MAX_MESSAGE)}\n… (shortened; the full text is in the job log)` : a.message;
  out(`::error ${props}::${esc(msg)}`);
}

export function parse(text) {
  const lines = text.split(/\r?\n/);
  const found = [];
  for (let i = 0; i < lines.length && found.length < MAX; i++) {
    const l = lines[i];
    let m;
    // cargo test: thread 'games::tests::x' (1234) panicked at src/games.rs:705:9:\n<message…>
    if ((m = /^thread '([^']+)'(?: \(\d+\))? panicked at (.+?):(\d+):(\d+):?\s*$/.exec(l))) {
      const msg = [];
      for (let j = i + 1; j < lines.length && msg.length < 12; j++) {
        if (/^(note: |stack backtrace:|\s+\d+: |thread '|test result:|failures:|---- )/.test(lines[j])) break;
        msg.push(lines[j]);
      }
      found.push({ file: rel(m[2]), line: m[3], col: m[4], title: `${label}: ${m[1]} failed`, message: msg.join('\n').trim() || 'panicked' });
      continue;
    }
    // rustc / clippy: error[E0425]: message \n  --> src/lib.rs:625:29
    if ((m = /^(error|warning)(\[[A-Z0-9]+\])?: (.+)$/.exec(l)) && !/^error: (could not compile|aborting|test failed|process didn't)/.test(l)) {
      const loc = lines.slice(i + 1, i + 4).map((x) => /-->\s+(.+?):(\d+):(\d+)/.exec(x)).find(Boolean);
      if (loc || m[1] === 'error') {
        const detail = lines.slice(i, i + 12).join('\n');
        found.push({ ...(loc ? { file: rel(loc[1]), line: loc[2], col: loc[3] } : {}), title: `${label}: ${m[3]}`.slice(0, 200), message: detail });
      }
      continue;
    }
    // cargo fmt --check: Diff in /path/src/lib.rs at line 12:  (or :12:)
    if ((m = /^Diff in (.+?)(?: at line |:)(\d+):?/.exec(l))) {
      found.push({ file: rel(m[1]), line: m[2], title: `${label}: not formatted (run cargo fmt)`, message: lines.slice(i, i + 10).join('\n') });
      continue;
    }
    // svelte-check / tsc: path:line:col \n Error: message   or   path(line,col): error TS…: message
    if ((m = /^(.+?\.(?:ts|tsx|js|mjs|svelte)):(\d+):(\d+)\s*$/.exec(l)) && /^(Error|Warn)/.test(lines[i + 1] ?? '')) {
      const msg = [];
      for (let j = i + 1; j < lines.length && msg.length < 7 && lines[j].trim() !== ''; j++) msg.push(lines[j]);
      found.push({ file: rel(m[1]), line: m[2], col: m[3], title: `${label}: ${lines[i + 1].slice(0, 160)}`, message: msg.join('\n') });
      continue;
    }
    if ((m = /^(.+?\.(?:ts|tsx|js|mjs|svelte))\((\d+),(\d+)\): error (TS\d+): (.+)$/.exec(l))) {
      found.push({ file: rel(m[1]), line: m[2], col: m[3], title: `${label}: ${m[4]}`, message: m[5] });
      continue;
    }
    // ESLint (stylish): a file line, then "  12:3  error  message  rule"
    if ((m = /^\s+(\d+):(\d+)\s+error\s+(.+?)\s{2,}(\S+)\s*$/.exec(l))) {
      let file = '';
      for (let j = i - 1; j >= 0; j--) {
        if (/^\S.*\.(ts|tsx|js|mjs|svelte|cjs)$/.test(lines[j])) {
          file = lines[j].trim();
          break;
        }
      }
      found.push({ ...(file ? { file: rel(file) } : {}), line: m[1], col: m[2], title: `${label}: ${m[4]}`, message: m[3] });
      continue;
    }
  }
  return found;
}

function annotate(text, code) {
  const found = parse(text);
  if (found.length === 0) {
    const tail = text.trim().split(/\r?\n/).slice(-25).join('\n');
    emit({ title: `${label} failed (exit code ${code})`, message: tail || '(no output)' });
    return;
  }
  for (const a of found) emit(a);
  out(`::notice title=${escProp(label)}::${found.length} failure(s) reported as annotations`);
}
