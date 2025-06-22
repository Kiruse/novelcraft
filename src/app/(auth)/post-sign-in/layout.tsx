import { ConvexSignalsAuthProvider } from '~/providers/ConvexSignalsAuthProvider';

export default function PostSignInLayout({ children }: { children: React.ReactNode }) {
  return (
    <ConvexSignalsAuthProvider>
      {children}
    </ConvexSignalsAuthProvider>
  );
}
