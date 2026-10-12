import { describe, expect, it } from 'vitest';
import { formatBytes, plural } from '../../src/lib/format';
import pkg from '../../package.json';

describe('format', () => {
  it('formats bytes', () => {
    expect(formatBytes(0)).toBe('0 bytes');
    expect(formatBytes(999)).toBe('999 bytes');
    expect(formatBytes(4_512_000_000)).toBe('4.5 GB');
    expect(formatBytes(980_000_000)).toBe('980 MB');
    expect(formatBytes(-1)).toBe('—');
  });
  it('pluralises', () => {
    expect(plural(1, 'game', 'games')).toBe('1 game');
    expect(plural(1234, 'file', 'files')).toBe('1,234 files');
  });
});

describe('version', () => {
  it('is a 4.0.0 pre-release during T0–T2', () => {
    expect(pkg.version).toMatch(/^4\.0\.0-alpha\.\d+$/);
  });
});
