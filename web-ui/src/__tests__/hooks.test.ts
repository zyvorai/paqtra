import { describe, it, expect } from 'vitest';
import { shortcuts } from '../hooks/useKeyboardShortcuts';

describe('useKeyboardShortcuts', () => {
  it('exports shortcuts array', () => {
    expect(Array.isArray(shortcuts)).toBe(true);
    expect(shortcuts.length).toBeGreaterThan(0);
  });

  it('shortcuts have required fields', () => {
    shortcuts.forEach((s) => {
      expect(s).toHaveProperty('keys');
      expect(s).toHaveProperty('description');
      expect(s).toHaveProperty('category');
      expect(typeof s.keys).toBe('string');
      expect(typeof s.description).toBe('string');
      expect(['Navigation', 'Actions', 'General']).toContain(s.category);
    });
  });

  it('includes help shortcut', () => {
    const help = shortcuts.find((s) => s.keys === '?');
    expect(help).toBeDefined();
    expect(help?.description).toContain('keyboard shortcuts');
  });

  it('includes search shortcut', () => {
    const search = shortcuts.find((s) => s.keys === '/');
    expect(search).toBeDefined();
  });

  it('includes navigation shortcuts', () => {
    const nav = shortcuts.filter((s) => s.category === 'Navigation');
    expect(nav.length).toBeGreaterThan(5);
  });
});
