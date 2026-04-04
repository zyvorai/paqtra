import React from 'react';
import { LineChart, Line, ResponsiveContainer } from 'recharts';

interface SparklineProps {
  data: number[];
  color?: string;
  width?: number;
  height?: number;
}

const Sparkline: React.FC<SparklineProps> = ({ data, color = '#3b82f6', width = 80, height = 24 }) => {
  const chartData = data.map((v, i) => ({ i, v }));
  return (
    <ResponsiveContainer width={width} height={height}>
      <LineChart data={chartData}>
        <Line type="monotone" dataKey="v" stroke={color} dot={false} strokeWidth={1.5} />
      </LineChart>
    </ResponsiveContainer>
  );
};

export default Sparkline;
