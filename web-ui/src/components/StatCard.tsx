import React from 'react';
import { TrendingUp, TrendingDown } from 'lucide-react';

const GLOW_MAP: Record<string, string> = {
  blue: 'card-glow',
  green: 'card-glow-green',
  purple: 'card-glow-purple',
  cyan: 'card-glow-cyan',
  red: 'card-glow',
  orange: 'card-glow',
};

const ICON_BG_MAP: Record<string, string> = {
  blue: 'bg-gradient-to-br from-blue-500 to-blue-700 shadow-blue-500/20',
  green: 'bg-gradient-to-br from-green-500 to-emerald-700 shadow-green-500/20',
  purple: 'bg-gradient-to-br from-purple-500 to-purple-700 shadow-purple-500/20',
  cyan: 'bg-gradient-to-br from-cyan-500 to-cyan-700 shadow-cyan-500/20',
  red: 'bg-gradient-to-br from-red-500 to-red-700 shadow-red-500/20',
  orange: 'bg-gradient-to-br from-orange-500 to-orange-700 shadow-orange-500/20',
};

const BADGE_MAP: Record<string, string> = {
  blue: 'bg-blue-500/10 text-blue-400',
  green: 'bg-green-500/10 text-green-400',
  purple: 'bg-purple-500/10 text-purple-400',
  cyan: 'bg-cyan-500/10 text-cyan-400',
  red: 'bg-red-500/10 text-red-400',
  orange: 'bg-orange-500/10 text-orange-400',
};

interface StatCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: React.ReactNode;
  color?: string;
  badge?: string;
  trend?: {
    value: number;
    isPositive: boolean;
  };
}

export function StatCard({
  title,
  value,
  subtitle,
  icon,
  color = 'blue',
  badge,
  trend,
}: StatCardProps) {
  const gradient = `stat-card-${color}`;
  const glow = GLOW_MAP[color] || 'card-glow';
  const iconBg = ICON_BG_MAP[color] || ICON_BG_MAP.blue;
  const badgeCls = BADGE_MAP[color] || BADGE_MAP.blue;

  return (
    <div className={`${gradient} rounded-xl p-5 border border-slate-700/50 ${glow} transition-all hover:scale-[1.02]`}>
      <div className="flex items-center justify-between mb-3">
        {icon && (
          <div className={`w-10 h-10 rounded-lg ${iconBg} flex items-center justify-center shadow-lg`}>
            {icon}
          </div>
        )}
        {badge && (
          <span className={`text-[10px] font-medium px-2 py-0.5 rounded-full ${badgeCls}`}>
            {badge}
          </span>
        )}
      </div>
      <div className="text-2xl font-bold text-white">{value}</div>
      <div className="text-xs text-slate-400 mt-1">{title}</div>
      {subtitle && (
        <div className="text-xs text-slate-500 mt-0.5">{subtitle}</div>
      )}
      {trend && (
        <div
          className={`flex items-center gap-1 mt-1 text-xs font-semibold ${
            trend.isPositive ? 'text-emerald-400' : 'text-red-400'
          }`}
        >
          {trend.isPositive ? (
            <TrendingUp className="w-3 h-3" />
          ) : (
            <TrendingDown className="w-3 h-3" />
          )}
          {Math.abs(trend.value).toFixed(1)}%
        </div>
      )}
    </div>
  );
}

export default StatCard;
