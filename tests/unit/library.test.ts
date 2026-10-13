import { describe, expect, it } from 'vitest';
import game1 from '../fixtures/library/wpf-game1.game.json';
import game2 from '../fixtures/library/wpf-game2.game.json';
import alpha4 from '../fixtures/library/alpha4.game.json';
import { SAMPLE_GAMES } from '../../src/lib/core/mock';

// B-6: the fixtures the core reads (src-tauri/src/games.rs) keep the real WPF field names.
describe('library fixtures', () => {
  it('the real WPF game.json files have the fields the loader reads', () => {
    for (const g of [game1, game2]) {
      for (const k of ['Name', 'Shortcut', 'Thumbnail', 'Background', 'Hero', 'Icon', 'Portrait', 'Favorite', 'Hidden', 'PlayCount', 'PlayTimeMinutes', 'LastSessionMinutes'])
        expect(Object.keys(g)).toContain(k);
    }
    expect(game2.Category).toBe('Steam');
    expect(alpha4.AioneSchema).toBe(1);
  });
  it('the mock library is like the copy: 10 games, none hidden', () => {
    expect(SAMPLE_GAMES).toHaveLength(10);
    expect(SAMPLE_GAMES.filter((g) => g.hidden)).toHaveLength(0);
  });
});
