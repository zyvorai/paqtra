import React from 'react';
import { BarChart3 } from 'lucide-react';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';

interface ChartContainerProps {
  title: string;
  type: 'line' | 'bar' | 'pie';
  data: Record<string, unknown>[];
  dataKeys?: string[];
  colors?: string[];
  xAxisKey?: string;
  height?: number;
  icon?: React.ReactNode;
}

const DEFAULT_COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899'];

const tooltipStyle = {
  backgroundColor: '#0f172a',
  border: '1px solid #1e293b',
  borderRadius: '0.75rem',
  fontSize: '12px',
  color: '#e2e8f0',
};

export const ChartContainer: React.FC<ChartContainerProps> = ({
  title,
  type,
  data,
  dataKeys = [],
  colors = DEFAULT_COLORS,
  xAxisKey = 'timestamp',
  height = 300,
  icon,
}) => {
  const renderChart = () => {
    switch (type) {
      case 'line':
        return (
          <ResponsiveContainer width="100%" height={height}>
            <LineChart data={data}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
              <XAxis
                dataKey={xAxisKey}
                stroke="#475569"
                fontSize={11}
                tickFormatter={(value) => {
                  if (xAxisKey === 'timestamp') {
                    const date = new Date(value);
                    return date.toLocaleTimeString();
                  }
                  return value;
                }}
              />
              <YAxis stroke="#475569" fontSize={11} />
              <Tooltip contentStyle={tooltipStyle} />
              <Legend />
              {dataKeys.map((key, index) => (
                <Line
                  key={key}
                  type="monotone"
                  dataKey={key}
                  stroke={colors[index % colors.length]}
                  strokeWidth={2}
                  dot={false}
                  animationDuration={300}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        );

      case 'bar':
        return (
          <ResponsiveContainer width="100%" height={height}>
            <BarChart data={data}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
              <XAxis dataKey={xAxisKey} stroke="#475569" fontSize={11} />
              <YAxis stroke="#475569" fontSize={11} />
              <Tooltip contentStyle={tooltipStyle} />
              <Legend />
              {dataKeys.map((key, index) => (
                <Bar
                  key={key}
                  dataKey={key}
                  fill={colors[index % colors.length]}
                  animationDuration={300}
                />
              ))}
            </BarChart>
          </ResponsiveContainer>
        );

      case 'pie':
        return (
          <ResponsiveContainer width="100%" height={height}>
            <PieChart>
              <Pie
                data={data}
                dataKey="value"
                nameKey="name"
                cx="50%"
                cy="50%"
                outerRadius={80}
                label
                animationDuration={300}
              >
                {data.map((_, index) => (
                  <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
                ))}
              </Pie>
              <Tooltip contentStyle={tooltipStyle} />
              <Legend />
            </PieChart>
          </ResponsiveContainer>
        );

      default:
        return null;
    }
  };

  return (
    <div className="bg-slate-800/50 rounded-xl p-5 border border-slate-700/50 border-t-2 border-t-blue-500/30">
      <h3 className="text-sm font-semibold text-white flex items-center gap-2 mb-4">
        {icon || <BarChart3 className="w-4 h-4 text-slate-400" />}
        {title}
      </h3>
      {data.length === 0 ? (
        <div
          className="flex items-center justify-center text-slate-500"
          style={{ height }}
        >
          No data available
        </div>
      ) : (
        renderChart()
      )}
    </div>
  );
};

export default ChartContainer;
