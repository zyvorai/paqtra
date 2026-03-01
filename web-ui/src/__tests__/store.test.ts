import { describe, it, expect } from 'vitest';
import { store, setConnected, setLastUpdate } from '../store';

describe('Redux Store', () => {
  it('has initial metrics state', () => {
    const state = store.getState();
    expect(state.metrics.connected).toBe(false);
    expect(state.metrics.lastUpdate).toBeNull();
  });

  it('setConnected updates state', () => {
    store.dispatch(setConnected(true));
    expect(store.getState().metrics.connected).toBe(true);
    store.dispatch(setConnected(false));
    expect(store.getState().metrics.connected).toBe(false);
  });

  it('setLastUpdate updates state', () => {
    const ts = '2026-03-01T12:00:00Z';
    store.dispatch(setLastUpdate(ts));
    expect(store.getState().metrics.lastUpdate).toBe(ts);
  });
});
