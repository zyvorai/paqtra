import { describe, it, expect } from 'vitest';
import api from '../services/api';

describe('API Client', () => {
  it('has correct default base URL', () => {
    expect(api.defaults.baseURL).toBe('/api/v1');
  });

  it('has correct timeout', () => {
    expect(api.defaults.timeout).toBe(15000);
  });

  it('has JSON content type header', () => {
    expect(api.defaults.headers['Content-Type']).toBe('application/json');
  });

  it('has request interceptor', () => {
    expect(api.interceptors.request.handlers.length).toBeGreaterThan(0);
  });

  it('has response interceptor', () => {
    expect(api.interceptors.response.handlers.length).toBeGreaterThan(0);
  });
});
