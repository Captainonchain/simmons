import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "SIMMONS — Autonomous AI Trading on X Layer",
  description: "Autonomous AI Trading on X Layer",
  icons: {
    icon: [
      { url: "/favicon.png", sizes: "32x32", type: "image/png" },
      { url: "/logo1.png", sizes: "192x192", type: "image/png" },
    ],
    apple: "/logo1.png",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" data-theme="dark">
      <head>
        <meta name="viewport" content="width=device-width, initial-scale=1" />
      </head>
      <body>{children}</body>
    </html>
  );
}
