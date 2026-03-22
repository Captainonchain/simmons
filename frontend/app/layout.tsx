import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "SIMMONS — Autonomous AI Trading on X Layer",
  description: "Autonomous AI Trading on X Layer",
  icons: {
    icon: "/logo1.png",
    apple: "/logo1.png",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" data-theme="dark">
      <body>{children}</body>
    </html>
  );
}
