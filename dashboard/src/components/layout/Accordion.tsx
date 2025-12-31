"use client";

import { useState, ReactNode, useId } from "react";

interface AccordionProps {
  title: string;
  icon: ReactNode;
  badge?: number | string;
  badgeVariant?: "default" | "warning" | "error";
  defaultOpen?: boolean;
  children: ReactNode;
  actions?: ReactNode;
  id?: string;
}

export function Accordion({ 
  title, 
  icon, 
  badge, 
  badgeVariant = "default",
  defaultOpen = false, 
  children,
  actions,
  id 
}: AccordionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const contentId = useId();

  const badgeColors = {
    default: "bg-emerald-500 text-white",
    warning: "bg-amber-500 text-white",
    error: "bg-red-500 text-white",
  };

  return (
    <section id={id} className="bg-[#131a26] border border-[#2a3548] rounded-xl overflow-hidden">
      <div className="flex items-center">
        <h2 className="flex-1">
          <button
            onClick={() => setIsOpen(!isOpen)}
            aria-expanded={isOpen}
            aria-controls={contentId}
            className="w-full flex items-center px-5 py-4 hover:bg-[#1a2332] transition-colors"
          >
            <svg 
              className={`w-5 h-5 mr-3 text-[#8b95a5] transition-transform duration-200 ${isOpen ? 'rotate-90' : ''}`}
              viewBox="0 0 24 24" 
              fill="none" 
              stroke="currentColor" 
              strokeWidth="2"
              aria-hidden="true"
            >
              <polyline points="9 18 15 12 9 6"/>
            </svg>
            
            <span className="w-5 h-5 mr-2.5 text-emerald-400" aria-hidden="true">
              {icon}
            </span>
            
            <span className="text-[13px] font-semibold uppercase tracking-wide text-white flex-1 text-left">
              {title}
            </span>
            
            {badge !== undefined && (
              <span className={`px-2 py-0.5 rounded-full text-[11px] font-semibold ${badgeColors[badgeVariant]}`}>
                {badge}
              </span>
            )}
          </button>
        </h2>
        {actions && (
          <div className="pr-5">
            {actions}
          </div>
        )}
      </div>
      
      <div 
        id={contentId}
        role="region"
        aria-labelledby={contentId}
        hidden={!isOpen}
        className={isOpen ? "px-5 pb-5" : ""}
      >
        {isOpen && children}
      </div>
    </section>
  );
}

