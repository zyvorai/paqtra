/** Theme-aware chart colors */
import { useThemeStore } from '../stores/themeStore';

export function useChartTheme() {
  const isDark = useThemeStore((s) => s.isDark);
  return {
    grid: isDark ? 'hsl(217 32% 17%)' : 'hsl(214 32% 91%)',
    tick: isDark ? '#94a3b8' : '#64748b',
    tooltipBg: isDark ? '#1e293b' : '#ffffff',
    tooltipBorder: isDark ? '1px solid #334155' : '1px solid #e2e8f0',
    text: isDark ? '#e2e8f0' : '#1e293b',
    muted: isDark ? '#64748b' : '#94a3b8',
  };
}
