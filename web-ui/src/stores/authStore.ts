import { create } from 'zustand';
import api from '../services/api';

interface AuthState {
  token: string | null;
  username: string | null;
  role: string | null;
  isAuthenticated: boolean;
  authRequired: boolean;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  checkSession: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  token: null,
  username: localStorage.getItem('cilium-vision-username'),
  role: null,
  isAuthenticated: false,
  authRequired: false,

  login: async (username: string, password: string) => {
    const response = await api.post('/auth/login', { username, password });
    const { token, role } = response.data;
    localStorage.setItem('cilium-vision-username', username);
    api.defaults.headers.common['Authorization'] = `Bearer ${token}`;
    set({ token, username, role: role ?? 'viewer', isAuthenticated: true });
  },

  logout: () => {
    localStorage.removeItem('cilium-vision-token');
    localStorage.removeItem('cilium-vision-username');
    delete api.defaults.headers.common['Authorization'];
    set({ token: null, username: null, role: null, isAuthenticated: false });
  },

  checkSession: async () => {
    try {
      // Use fetch directly to hit /health (not /api/v1/health)
      const resp = await fetch('/health');
      if (resp.status === 401) {
        set({ authRequired: true, isAuthenticated: false });
        localStorage.removeItem('cilium-vision-token');
      } else {
        set({ authRequired: false });
      }
    } catch {
      // Network error — API unreachable, don't require auth (show app anyway)
      set({ authRequired: false });
    }
  },
}));
