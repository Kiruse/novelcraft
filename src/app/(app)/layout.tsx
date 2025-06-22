import type { Metadata } from "next";
import { Footer } from '~/components/Footer';
import Header from '~/components/Header';
import { ConvexSignalsAuthProvider } from '~/providers/ConvexSignalsAuthProvider';

export const metadata: Metadata = {
  title: "NovelCraft",
  description: "NovelCraft, experimental Agentic Visual Novel Platform",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <ConvexSignalsAuthProvider>
      <Header />
      <main>{children}</main>
      <Footer />
    </ConvexSignalsAuthProvider>
  );
}
