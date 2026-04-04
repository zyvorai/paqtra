import React from 'react';

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  text?: string;
  fullScreen?: boolean;
}

const SIZES = { sm: 'w-4 h-4', md: 'w-8 h-8', lg: 'w-12 h-12' };
const BORDER_SIZES = { sm: 'border-2', md: 'border-2', lg: 'border-[3px]' };

const LoadingSpinner: React.FC<LoadingSpinnerProps> = ({ size = 'md', text, fullScreen }) => {
  const content = (
    <div className="flex flex-col items-center gap-3">
      <div className={`${SIZES[size]} ${BORDER_SIZES[size]} border-blue-500 border-t-transparent rounded-full animate-spin`} />
      {text && <span className="text-sm font-semibold text-slate-300 tracking-wide">{text}</span>}
    </div>
  );

  if (fullScreen) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-slate-950">
        {content}
      </div>
    );
  }

  return <div className="flex items-center justify-center py-12">{content}</div>;
};

export default LoadingSpinner;
