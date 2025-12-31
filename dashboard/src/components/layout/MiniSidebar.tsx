"use client";

import { useState } from "react";

interface NavItem {
  icon: React.ReactNode;
  label: string;
  sectionId: string;
}

const navItems: NavItem[] = [
  { 
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
        <polyline points="9 22 9 12 15 12 15 22"/>
      </svg>
    ), 
    label: "Dashboard", 
    sectionId: "top"
  },
  { 
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
        <line x1="12" y1="9" x2="12" y2="13"/>
        <line x1="12" y1="17" x2="12.01" y2="17"/>
      </svg>
    ), 
    label: "Alertas", 
    sectionId: "alerts"
  },
  { 
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
        <line x1="8" y1="21" x2="16" y2="21"/>
        <line x1="12" y1="17" x2="12" y2="21"/>
      </svg>
    ), 
    label: "Servidores", 
    sectionId: "infrastructure"
  },
  { 
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="2" y1="12" x2="22" y2="12"/>
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
      </svg>
    ), 
    label: "Red", 
    sectionId: "network"
  },
  { 
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M12 2a10 10 0 1 0 10 10H12V2z"/>
        <path d="M12 2a10 10 0 0 1 10 10"/>
      </svg>
    ), 
    label: "IA", 
    sectionId: "ai"
  },
];

interface MiniSidebarProps {
  activeSection?: string;
  onSectionClick?: (sectionId: string) => void;
}

export function MiniSidebar({ activeSection = "top", onSectionClick }: MiniSidebarProps) {
  const [hoveredItem, setHoveredItem] = useState<string | null>(null);

  const handleClick = (sectionId: string) => {
    if (onSectionClick) {
      onSectionClick(sectionId);
    }
    
    const element = document.getElementById(sectionId);
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  };

  return (
    <aside className="fixed left-0 top-0 h-screen w-14 bg-[#131a26] border-r border-[#2a3548] flex flex-col items-center py-3 z-50">
      <button 
        onClick={() => handleClick("top")}
        className="w-9 h-9 bg-gradient-to-br from-emerald-400 to-indigo-500 rounded-lg flex items-center justify-center text-white font-bold text-sm mb-6"
      >
        C
      </button>
      
      <nav className="flex flex-col gap-1 flex-1">
        {navItems.map((item) => {
          const isActive = activeSection === item.sectionId;
          
          return (
            <button
              key={item.sectionId}
              onClick={() => handleClick(item.sectionId)}
              onMouseEnter={() => setHoveredItem(item.sectionId)}
              onMouseLeave={() => setHoveredItem(null)}
              className={`relative w-10 h-10 flex items-center justify-center rounded-lg transition-colors ${
                isActive 
                  ? "bg-emerald-500/15 text-emerald-400" 
                  : "text-[#8b95a5] hover:bg-[#1a2332] hover:text-white"
              }`}
            >
              <span className="w-5 h-5">{item.icon}</span>
              
              {hoveredItem === item.sectionId && (
                <span className="absolute left-14 bg-[#131a26] border border-[#2a3548] px-3 py-1.5 rounded-md text-[12px] text-white whitespace-nowrap z-50 shadow-lg">
                  {item.label}
                </span>
              )}
            </button>
          );
        })}
      </nav>
      
      <div className="w-8 h-8 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white text-[12px] font-semibold cursor-pointer">
        A
      </div>
    </aside>
  );
}

