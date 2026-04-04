import React from 'react';
import { Inbox } from 'lucide-react';

interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: React.ReactNode;
}

const EmptyState: React.FC<EmptyStateProps> = ({ icon, title, description, action }) => (
  <div className="flex flex-col items-center justify-center py-16 text-center">
    <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-slate-700 to-slate-800 flex items-center justify-center mx-auto mb-4 shadow-lg">
      {icon || <Inbox className="w-8 h-8 text-slate-400" />}
    </div>
    <h3 className="text-lg font-medium text-white mb-2">{title}</h3>
    {description && <p className="text-sm text-slate-400 max-w-md mb-6">{description}</p>}
    {action}
  </div>
);

export default EmptyState;
