import type { Metadata } from "next";
import { Footer } from '~/components/Footer';
import Header from '~/components/Header';

export const metadata: Metadata = {
  title: "NovelCraft",
  description: "NovelCraft, experimental Agentic Visual Novel Platform",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <>
      <Header />
      <main data-mdx="document" className="max-w-screen-md mx-auto">{children}</main>
      <Footer />
    </>
  );
}
