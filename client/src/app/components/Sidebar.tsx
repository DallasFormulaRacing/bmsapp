import { LayoutDashboard, Grid3x3, Info, Activity, LineChart } from 'lucide-react';

interface SidebarProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
}

export function Sidebar({ activeTab, onTabChange }: SidebarProps) {
  const tabs = [
    { id: 'overview', icon: LayoutDashboard, label: 'Overview' },
    { id: 'segments', icon: Grid3x3, label: 'Segments' },
    { id: 'analytics', icon: LineChart, label: 'Analytics' },
    { id: 'status', icon: Activity, label: 'Status' },
    { id: 'info', icon: Info, label: 'Info' },
  ];

  return (
    <aside className="w-16 bg-card border-r border-border flex flex-col items-center py-6 gap-6 backdrop-blur-xl bg-opacity-80">
      {tabs.map(({ id, icon: Icon, label }) => (
        <button
          key={id}
          onClick={() => onTabChange(id)}
          className={`p-3 rounded-lg transition-all duration-300 ${
            activeTab === id 
              ? 'bg-primary text-primary-foreground' 
              : 'text-muted-foreground hover:bg-accent/50 hover:text-primary'
          }`}
          title={label}
        >
          <Icon className="w-5 h-5" />
        </button>
      ))}
    </aside>
  );
}