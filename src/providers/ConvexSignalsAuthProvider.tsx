'use client';

import { useClerk } from '@clerk/nextjs';
import { createContext, useLayoutEffect } from 'react';
import { convex } from '~/config';

const never = null as any as never;
const Context = createContext<never>(never);

export function ConvexSignalsAuthProvider({ children }: { children: React.ReactNode }) {
  const clerk = useClerk();

  useLayoutEffect(() => {
    convex.setAuth(async ({ forceRefreshToken }) => {
      if (!clerk.loaded) {
        await new Promise((resolve, reject) => {
          clerk.on('status', (e) => {
            if (e === 'ready') {
              resolve(true);
            } else if (e === 'error') {
              reject(new Error('Clerk failed to load'));
            }
          });
        });
      }
      return await clerk.session?.getToken({ skipCache: forceRefreshToken, template: 'convex' });
    });
  }, [clerk]);

  return (
    <Context.Provider value={never}>
      {children}
    </Context.Provider>
  );
}
