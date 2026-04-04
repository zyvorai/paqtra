import { describe, it, expect } from 'vitest';
import { t } from '../i18n';

describe('i18n', () => {
  it('returns English translation for known key', () => {
    expect(t('dashboard')).toBe('Dashboard');
    expect(t('flows')).toBe('Flows');
    expect(t('settings')).toBe('Settings');
  });

  it('returns key itself for unknown key', () => {
    expect(t('nonexistent_key_xyz')).toBe('nonexistent_key_xyz');
  });

  it('falls back to English for empty locale', () => {
    expect(t('dashboard', 'es')).toBe('Dashboard');
  });

  it('returns key for truly unknown key in any locale', () => {
    expect(t('nonexistent_xyz', 'es')).toBe('nonexistent_xyz');
  });

  it('handles common action labels', () => {
    expect(t('save')).toBe('Save');
    expect(t('cancel')).toBe('Cancel');
    expect(t('delete')).toBe('Delete');
    expect(t('create')).toBe('Create');
    expect(t('refresh')).toBe('Refresh');
    expect(t('search')).toBe('Search');
  });

  it('handles status labels', () => {
    expect(t('loading')).toBe('Loading...');
    expect(t('error')).toBe('Error');
    expect(t('success')).toBe('Success');
    expect(t('noData')).toBe('No data available');
  });
});
