import { create } from 'zustand';

interface ThemeState {
  isDark: boolean;
  toggle: () => void;
}

export const useThemeStore = create<ThemeState>((set) => ({
  isDark: localStorage.getItem('cilium-vision-theme') !== 'light',
  toggle: () =>
    set((state) => {
      const next = !state.isDark;
      localStorage.setItem('cilium-vision-theme', next ? 'dark' : 'light');
      document.documentElement.classList.toggle('dark', next);
      document.documentElement.classList.toggle('light-theme', !next);
      return { isDark: next };
    }),
}));
