"use client";

import { useState, ReactNode, useEffect, useRef, useCallback } from "react";

interface SidePanelProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  defaultWidth?: number;
  minWidth?: number;
  maxWidth?: number;
}

export function SidePanel({ 
  isOpen, 
  onClose, 
  title, 
  children,
  defaultWidth = 500,
  minWidth = 400,
  maxWidth = 1800,
}: SidePanelProps) {
  const [width, setWidth] = useState(defaultWidth);
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    
    if (isOpen) {
      document.addEventListener("keydown", handleEscape);
      document.body.style.overflow = "hidden";
    }
    
    return () => {
      document.removeEventListener("keydown", handleEscape);
      document.body.style.overflow = "";
    };
  }, [isOpen, onClose]);

  const startResizing = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  const stopResizing = useCallback(() => {
    setIsResizing(false);
  }, []);

  const resize = useCallback((e: MouseEvent) => {
    if (isResizing) {
      const newWidth = window.innerWidth - e.clientX;
      if (newWidth >= minWidth && newWidth <= maxWidth) {
        setWidth(newWidth);
      }
    }
  }, [isResizing, minWidth, maxWidth]);

  useEffect(() => {
    if (isResizing) {
      window.addEventListener("mousemove", resize);
      window.addEventListener("mouseup", stopResizing);
      document.body.style.cursor = "ew-resize";
      document.body.style.userSelect = "none";
    }

    return () => {
      window.removeEventListener("mousemove", resize);
      window.removeEventListener("mouseup", stopResizing);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing, resize, stopResizing]);

  // Reset width when panel opens
  useEffect(() => {
    if (isOpen) {
      setWidth(defaultWidth);
    }
  }, [isOpen, defaultWidth]);

  if (!isOpen) return null;

  return (
    <>
      <div 
        className="fixed inset-0 bg-black/50 z-40 transition-opacity"
        onClick={onClose}
      />
      <div 
        ref={panelRef}
        className="fixed top-0 right-0 h-full bg-[#131a26] border-l border-[#2a3548] z-50 flex flex-col shadow-xl"
        style={{ width: `${width}px` }}
      >
        {/* Resize handle - positioned below header */}
        <div
          className={`absolute left-0 top-[60px] w-2 h-[calc(100%-60px)] cursor-ew-resize group ${
            isResizing ? "bg-emerald-500/30" : ""
          }`}
          onMouseDown={startResizing}
        >
          {/* Visual indicator */}
          <div className={`absolute left-0 top-0 w-1 h-full transition-colors ${
            isResizing ? "bg-emerald-500" : "bg-transparent group-hover:bg-emerald-500/50"
          }`} />
          {/* Drag handle icon in the middle */}
          <div className={`absolute left-0 top-1/2 -translate-y-1/2 w-2 h-12 flex items-center justify-center transition-opacity ${
            isResizing ? "opacity-100" : "opacity-0 group-hover:opacity-100"
          }`}>
            <div className="flex flex-col gap-0.5">
              <div className="w-0.5 h-0.5 rounded-full bg-emerald-400" />
              <div className="w-0.5 h-0.5 rounded-full bg-emerald-400" />
              <div className="w-0.5 h-0.5 rounded-full bg-emerald-400" />
              <div className="w-0.5 h-0.5 rounded-full bg-emerald-400" />
              <div className="w-0.5 h-0.5 rounded-full bg-emerald-400" />
            </div>
          </div>
        </div>

        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[#2a3548]">
          <h2 className="text-[16px] font-semibold text-white">{title}</h2>
          <button 
            onClick={onClose}
            className="w-8 h-8 flex items-center justify-center rounded-md hover:bg-[#1a2332] text-[#8b95a5] hover:text-white transition-colors"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-5">
          {children}
        </div>
      </div>
    </>
  );
}

interface PanelTabsProps {
  tabs: string[];
  activeTab: string;
  onChange: (tab: string) => void;
}

export function PanelTabs({ tabs, activeTab, onChange }: PanelTabsProps) {
  return (
    <div className="flex border-b border-[#2a3548] -mx-5 px-5 mb-5">
      {tabs.map((tab) => (
        <button
          key={tab}
          onClick={() => onChange(tab)}
          className={`flex-1 py-3 text-[13px] border-b-2 transition-colors ${
            activeTab === tab 
              ? "text-emerald-400 border-emerald-400" 
              : "text-[#8b95a5] border-transparent hover:text-white"
          }`}
        >
          {tab}
        </button>
      ))}
    </div>
  );
}
