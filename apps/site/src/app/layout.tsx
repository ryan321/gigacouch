import type { Metadata, Viewport } from "next";
import type { ReactNode } from "react";
import "./globals.css";

export const metadata: Metadata = {
  title: { default: "Giga Couch", template: "%s – Giga Couch" },
  description: "Upload web games and play them in Chrome with a controller.",
  icons: { icon: "/brand/mark.png" },
};

export const viewport: Viewport = { themeColor: "#101722", colorScheme: "dark" };

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
