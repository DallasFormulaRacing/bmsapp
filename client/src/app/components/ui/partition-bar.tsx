import * as React from "react";

interface PartitionBarProps {
  size?: "sm" | "md" | "lg";
  children: React.ReactNode;
}

interface PartitionBarSegmentProps {
  num: number;
  variant?: "default" | "secondary" | "success" | "warning" | "danger";
  children: React.ReactNode;
}

interface PartitionBarSegmentTitleProps {
  children: React.ReactNode;
}

interface PartitionBarSegmentValueProps {
  children: React.ReactNode;
}

const PartitionBarContext = React.createContext<{ total: number }>({ total: 0 });

export default function PartitionBar({ size = "md", children }: PartitionBarProps) {
  const total = React.useMemo(() => {
    return React.Children.toArray(children).reduce((sum, child) => {
      if (React.isValidElement<PartitionBarSegmentProps>(child)) {
        return sum + (child.props.num || 0);
      }
      return sum;
    }, 0);
  }, [children]);

  const heightClass = {
    sm: "h-6",
    md: "h-8",
    lg: "h-10",
  }[size];

  return (
    <PartitionBarContext.Provider value={{ total }}>
      <div className="w-full space-y-2">
        <div className={`w-full ${heightClass} flex overflow-hidden rounded`}>
          {children}
        </div>
      </div>
    </PartitionBarContext.Provider>
  );
}

export function PartitionBarSegment({ num, variant = "default", children }: PartitionBarSegmentProps) {
  const { total } = React.useContext(PartitionBarContext);
  const percentage = total > 0 ? (num / total) * 100 : 0;

  const variantClasses = {
    default: "bg-primary/70 hover:bg-primary/80",
    secondary: "bg-muted hover:bg-muted/80",
    success: "bg-green-500/70 hover:bg-green-500/80",
    warning: "bg-orange-500/70 hover:bg-orange-500/80",
    danger: "bg-red-500/70 hover:bg-red-500/80",
  }[variant];

  const [title, value] = React.useMemo(() => {
    const childArray = React.Children.toArray(children);
    const titleChild = childArray.find(
      (child) => React.isValidElement(child) && child.type === PartitionBarSegmentTitle
    );
    const valueChild = childArray.find(
      (child) => React.isValidElement(child) && child.type === PartitionBarSegmentValue
    );
    return [titleChild, valueChild];
  }, [children]);

  return (
    <div
      className={`${variantClasses} flex items-center justify-center transition-all duration-300 relative group`}
      style={{ width: `${percentage}%` }}
    >
      {/* Tooltip on hover */}
      <div className="absolute bottom-full mb-2 hidden group-hover:block bg-black/90 text-white text-xs rounded px-2 py-1 whitespace-nowrap z-10">
        {title && React.isValidElement(title) ? title.props.children : ''}: {num} ({percentage.toFixed(1)}%)
      </div>
      
      {/* Content - show value if segment is wide enough, or as overlay for small segments */}
      {percentage > 8 ? (
        <div className="text-xs font-medium text-white px-2 text-center whitespace-nowrap">
          {value}
        </div>
      ) : (
        <div className="absolute inset-0 flex items-center justify-end pr-1">
          <div className="text-[10px] font-medium text-white whitespace-nowrap">
            {value}
          </div>
        </div>
      )}
    </div>
  );
}

export function PartitionBarSegmentTitle({ children }: PartitionBarSegmentTitleProps) {
  return <>{children}</>;
}

export function PartitionBarSegmentValue({ children }: PartitionBarSegmentValueProps) {
  return <>{children}</>;
}