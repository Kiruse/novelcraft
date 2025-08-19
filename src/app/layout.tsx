import type { Metadata } from "next";
import "./globals.sass";

export const metadata: Metadata = {
  title: "NovelCraft",
  description: "NovelCraft, experimental Agentic Visual Novel Platform",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className="flex flex-col max-w min-h-screen overflow-x-hidden">
        {children}
      </body>
    </html>
  );
}
