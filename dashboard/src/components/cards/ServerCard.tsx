"use client";

interface ServerCardProps {
  name: string;
  type?: string;
  status: "online" | "offline" | "warning";
  cpu: number;
  memory: number;
  lastSeen: string;
  onClick?: () => void;
}

export function ServerCard({ name, type, status, cpu, memory, lastSeen, onClick }: ServerCardProps) {
  const statusColors = {
    online: "bg-emerald-500",
    offline: "bg-red-500",
    warning: "bg-amber-500",
  };

  const getBarColor = (value: number, type: "cpu" | "memory") => {
    if (value > 80) return "bg-gradient-to-r from-amber-500 to-red-500";
    if (type === "cpu") return "bg-gradient-to-r from-emerald-400 to-indigo-500";
    return "bg-gradient-to-r from-indigo-500 to-purple-500";
  };

  return (
    <article 
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && onClick?.()}
      aria-label={`${name} - ${status} - CPU ${cpu}% RAM ${memory}%`}
      className="bg-[#0a0e17] border border-[#2a3548] rounded-lg p-4 cursor-pointer transition-all hover:border-emerald-400 hover:shadow-[0_0_0_1px_#00d4aa]"
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2 text-[14px] font-semibold text-white">
          <span className={`w-2 h-2 rounded-full ${statusColors[status]}`} />
          {name}
        </div>
        {type && (
          <span className="text-[11px] text-[#8b95a5] bg-[#1a2332] px-2 py-0.5 rounded">
            {type}
          </span>
        )}
      </div>
      
      <div className="space-y-2">
        <div className="flex items-center gap-3">
          <span className="text-[12px] text-[#8b95a5] w-9">CPU</span>
          <div className="flex-1 h-1.5 bg-[#2a3548] rounded-full overflow-hidden">
            <div 
              className={`h-full rounded-full transition-all duration-300 ${getBarColor(cpu, "cpu")}`}
              style={{ width: status === "offline" ? "0%" : `${cpu}%` }}
            />
          </div>
          <span className="text-[12px] font-medium text-white w-10 text-right">
            {status === "offline" ? "--" : `${cpu}%`}
          </span>
        </div>
        
        <div className="flex items-center gap-3">
          <span className="text-[12px] text-[#8b95a5] w-9">RAM</span>
          <div className="flex-1 h-1.5 bg-[#2a3548] rounded-full overflow-hidden">
            <div 
              className={`h-full rounded-full transition-all duration-300 ${getBarColor(memory, "memory")}`}
              style={{ width: status === "offline" ? "0%" : `${memory}%` }}
            />
          </div>
          <span className="text-[12px] font-medium text-white w-10 text-right">
            {status === "offline" ? "--" : `${memory}%`}
          </span>
        </div>
      </div>
    </article>
  );
}

