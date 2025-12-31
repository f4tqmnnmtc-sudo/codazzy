import "@/css/satoshi.css";
import "@/css/style.css";

import type { Metadata } from "next";
import NextTopLoader from "nextjs-toploader";
import type { PropsWithChildren } from "react";

export const metadata: Metadata = {
  title: {
    template: "%s | Codazzy",
    default: "Codazzy - Sistema de Monitorizacion",
  },
  description: "Sistema de monitorizacion predictiva para infraestructura IT.",
};

export default function RootLayout({ children }: PropsWithChildren) {
  return (
    <html lang="es">
      <body className="bg-[#0a0e17]">
        <NextTopLoader color="#00d4aa" showSpinner={false} />
        {children}
      </body>
    </html>
  );
}
