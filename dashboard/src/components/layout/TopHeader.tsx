"use client";

export function TopHeader() {
  return (
    <header className="h-14 bg-[#131a26] border-b border-[#2a3548] flex items-center px-6 gap-6">
      <div className="flex items-center gap-2 text-[13px] text-[#8b95a5]">
        <span>Codazzy</span>
        <span>/</span>
        <span className="text-white font-medium">Dashboard</span>
      </div>
    </header>
  );
}
