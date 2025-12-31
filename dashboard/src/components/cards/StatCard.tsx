"use client";

import { ReactNode } from "react";

interface StatCardProps {
  label: string;
  value: string | number;
  valueColor?: "default" | "success" | "warning" | "error";
  change?: { value: string; direction: "up" | "down" };
  children?: ReactNode;
}

export function StatCard({ label, value, valueColor = "default", change, children }: StatCardProps) {
  const valueColors = {
    default: "text-white",
    success: "text-emerald-400",
    warning: "text-amber-400",
    error: "text-red-400",
  };

  return (
    <div className="bg-[#131a26] border border-[#2a3548] rounded-xl p-5">
      <div className="flex items-center justify-between mb-2">
        <span className="text-[12px] text-[#8b95a5] uppercase tracking-wide">
          {label}
        </span>
        {change && (
          <span className={`text-[12px] flex items-center gap-1 ${change.direction === 'up' ? 'text-emerald-400' : 'text-red-400'}`}>
            {change.direction === 'up' ? '↑' : '↓'} {change.value}
          </span>
        )}
      </div>
      <div className={`text-[32px] font-semibold ${valueColors[valueColor]}`}>
        {value}
      </div>
      {children}
    </div>
  );
}

interface GaugeCardProps {
  label: string;
  value: number;
  color?: string;
  change?: { value: string; direction: "up" | "down" };
}

export function GaugeCard({ label, value, color = "#00d4aa", change }: GaugeCardProps) {
  const circumference = 2 * Math.PI * 26;
  const offset = circumference - (value / 100) * circumference;

  return (
    <div className="bg-[#131a26] border border-[#2a3548] rounded-xl p-5">
      <div className="flex items-center justify-between mb-2">
        <span className="text-[12px] text-[#8b95a5] uppercase tracking-wide">
          {label}
        </span>
      </div>
      <div className="flex items-center gap-4">
        <div className="relative w-16 h-16">
          <svg width="64" height="64" viewBox="0 0 64 64" className="-rotate-90">
            <circle 
              cx="32" 
              cy="32" 
              r="26" 
              fill="none" 
              stroke="#2a3548" 
              strokeWidth="6"
            />
            <circle 
              cx="32" 
              cy="32" 
              r="26" 
              fill="none" 
              stroke={color}
              strokeWidth="6"
              strokeLinecap="round"
              strokeDasharray={circumference}
              strokeDashoffset={offset}
              className="transition-all duration-500"
            />
          </svg>
          <div className="absolute inset-0 flex items-center justify-center text-[14px] font-semibold text-white">
            {value}%
          </div>
        </div>
        {change && (
          <div>
            <div className={`text-[12px] ${change.direction === 'up' ? 'text-emerald-400' : 'text-red-400'}`}>
              {change.direction === 'up' ? '↑' : '↓'} {change.value}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

