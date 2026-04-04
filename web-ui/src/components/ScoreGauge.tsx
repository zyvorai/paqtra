import React from 'react';

interface ScoreGaugeProps {
  score: number;
  max?: number;
  size?: number;
  label?: string;
}

function scoreColor(score: number): string {
  if (score >= 80) return '#22c55e';
  if (score >= 60) return '#eab308';
  return '#ef4444';
}

function scoreTextClass(score: number): string {
  if (score >= 80) return 'text-green-400';
  if (score >= 60) return 'text-yellow-400';
  return 'text-red-400';
}

const ScoreGauge: React.FC<ScoreGaugeProps> = ({ score, max = 100, size = 120, label }) => {
  const pct = max > 0 ? Math.min(score / max, 1) : 0;
  const r = (size / 2) - 8;
  const circumference = 2 * Math.PI * r;

  return (
    <div className="relative inline-flex items-center justify-center" style={{ width: size, height: size }}>
      <svg className="w-full h-full -rotate-90" viewBox={`0 0 ${size} ${size}`}>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="#334155"
          strokeWidth="8"
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={scoreColor(score)}
          strokeWidth="8"
          strokeDasharray={`${pct * circumference} ${circumference}`}
          strokeLinecap="round"
          className="transition-all duration-500"
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className={`text-3xl font-bold ${scoreTextClass(score)}`}>{Math.round(score)}</span>
        {label && <span className="text-xs text-slate-400">{label}</span>}
      </div>
    </div>
  );
};

export default ScoreGauge;
