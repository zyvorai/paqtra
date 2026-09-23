import { create } from 'zustand';
import api from '../services/api';

const TOKEN_KEY = 'paqtra-token';
const USER_KEY = 'paqtra-username';

interface AuthState {
  token: string | null;
  username: string | null;
  role: string | null;
  isAuthenticated: boolean;
  /** True until the first session probe finishes (Netra-style gate). */
  sessionReady: boolean;
  /** Show the login page when true. */
  authRequired: boolean;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  checkSession: () => Promise<void>;
}

function clearAuthHeaders() {
  delete api.defaults.headers.common['Authorization'];
}

export const useAuthStore = create<AuthState>((set) => ({
  token: localStorage.getItem(TOKEN_KEY),
  username: localStorage.getItem(USER_KEY),
  role: null,
  isAuthenticated: false,
  sessionReady: false,
  authRequired: true,

  login: async (username: string, password: string) => {
    const response = await api.post('/auth/login', { username, password });
    const { token, role } = response.data;
    localStorage.setItem(TOKEN_KEY, token);
    localStorage.setItem(USER_KEY, username);
    api.defaults.headers.common['Authorization'] = `Bearer ${token}`;
    set({
      token,
      username,
      role: role ?? 'viewer',
      isAuthenticated: true,
      authRequired: false,
      sessionReady: true,
    });
  },

  logout: () => {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
    clearAuthHeaders();
    set({
      token: null,
      username: null,
      role: null,
      isAuthenticated: false,
      authRequired: true,
      sessionReady: true,
    });
  },

  checkSession: async () => {
    const token = localStorage.getItem(TOKEN_KEY);
    if (!token) {
      clearAuthHeaders();
      set({
        token: null,
        isAuthenticated: false,
        authRequired: true,
        sessionReady: true,
      });
      return;
    }

    api.defaults.headers.common['Authorization'] = `Bearer ${token}`;
    try {
      // Probe a protected JSON endpoint (not /health — that is auth-exempt).
      const resp = await fetch('/api/v1/nodes', {
        headers: {
          Authorization: `Bearer ${token}`,
          Accept: 'application/json',
        },
      });
      if (resp.status === 401) {
        localStorage.removeItem(TOKEN_KEY);
        clearAuthHeaders();
        set({
          token: null,
          isAuthenticated: false,
          authRequired: true,
          sessionReady: true,
        });
        return;
      }
      set({
        token,
        username: localStorage.getItem(USER_KEY),
        isAuthenticated: true,
        authRequired: false,
        sessionReady: true,
      });
    } catch {
      // API unreachable — still show login so the gate is visible.
      set({
        isAuthenticated: false,
        authRequired: true,
        sessionReady: true,
      });
    }
  },
}));
