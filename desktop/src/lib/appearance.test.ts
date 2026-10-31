import { describe, it, expect } from 'vitest';
import { contrast, defaultAppearance, presets, themeTokens } from './appearance';
describe('Readable personal themes', () => {
  it('keeps critical text readable across presets, colors and modes', () => {
    for (const preset of presets)
      for (const dark of [true, false])
        for (const primary of [
          null,
          '#ffffff',
          '#000000',
          '#ffff00',
          '#c82d2b',
          '#087b83',
          '#7954aa',
        ]) {
          const tokens = themeTokens(
            { ...defaultAppearance().config, preset: preset.id, primary },
            dark,
          );
          for (const foreground of [
            '--foreground',
            '--subtle',
            '--muted-foreground',
            '--accent-ink',
          ] as const)
            expect(
              contrast(tokens[foreground], tokens['--surface']),
              `${preset.id}/${dark}/${primary}/${foreground}`,
            ).toBeGreaterThanOrEqual(4.5);
          expect(
            contrast(tokens['--primary'], tokens['--primary-foreground']),
          ).toBeGreaterThanOrEqual(4.5);
        }
  });
  it('provides comfortable defaults for the operational user independently', () => {
    expect(defaultAppearance(1).config.fontScale).toBe(110);
    expect(defaultAppearance(0).config.sidebar).toBe('expanded');
    const changed = defaultAppearance(1);
    changed.config.primary = '#123456';
    expect(defaultAppearance(0).config.primary).toBeNull();
  });
});
