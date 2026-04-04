import { describe, it, expect, beforeEach } from 'vitest';
import { useThemeStore } from '../stores/themeStore';
import { useAuthStore } from '../stores/authStore';
import { usePreferencesStore } from '../stores/preferencesStore';

describe('Theme Store', () => {
  it('has initial dark mode state', () => {
    expect(typeof useThemeStore.getState().isDark).toBe('boolean');
  });

  it('toggle switches theme', () => {
    const initial = useThemeStore.getState().isDark;
    useThemeStore.getState().toggle();
    expect(useThemeStore.getState().isDark).toBe(!initial);
    useThemeStore.getState().toggle(); // reset
  });
});

describe('Auth Store', () => {
  it('has initial state', () => {
    const state = useAuthStore.getState();
    expect(state.authRequired).toBe(false);
    expect(typeof state.login).toBe('function');
    expect(typeof state.logout).toBe('function');
    expect(typeof state.checkSession).toBe('function');
  });

  it('logout clears token', () => {
    useAuthStore.getState().logout();
    expect(useAuthStore.getState().token).toBeNull();
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
  });
});

describe('Preferences Store', () => {
  beforeEach(() => {
    usePreferencesStore.getState().resetPrefs();
  });

  it('has default values', () => {
    const state = usePreferencesStore.getState();
    expect(state.tablePageSize).toBe(25);
    expect(state.autoRefresh).toBe(false);
    expect(state.refreshInterval).toBe(10);
    expect(state.compactMode).toBe(false);
    expect(state.showTimestamps).toBe(true);
    expect(state.notifications).toBe(false);
    expect(state.language).toBe('en');
  });

  it('setPref updates a preference', () => {
    usePreferencesStore.getState().setPref('tablePageSize', 50);
    expect(usePreferencesStore.getState().tablePageSize).toBe(50);
  });

  it('setPref updates language', () => {
    usePreferencesStore.getState().setPref('language', 'es');
    expect(usePreferencesStore.getState().language).toBe('es');
  });

  it('resetPrefs restores defaults', () => {
    usePreferencesStore.getState().setPref('compactMode', true);
    usePreferencesStore.getState().setPref('tablePageSize', 100);
    usePreferencesStore.getState().resetPrefs();
    expect(usePreferencesStore.getState().compactMode).toBe(false);
    expect(usePreferencesStore.getState().tablePageSize).toBe(25);
  });
});
