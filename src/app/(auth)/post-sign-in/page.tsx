'use client';

import { useSignalEffect } from '@preact/signals-react';
import { useRouter } from 'next/navigation';
import { Spinner } from '~/components/Spinner';
import { convex } from '~/config';
import { api } from '~convex/_generated/api';

export default function PostSignInPage() {
  const router = useRouter();

  useSignalEffect(() => {
    console.log('convex.authenticated.value', convex.authenticated.value);
    if (!convex.authenticated.value) return;

    let mounted = true;
    convex.mutation(api.users.upsert).then(() => {
      if (!mounted) return;
      router.push('/dashboard');
    });

    return () => {
      mounted = false;
    };
  });

  return (
    <>
      <div className="text-center">
        <Spinner size="lg" className="mb-4" />
        <h2 className="text-xl font-semibold text-gray-200 mb-2">
          One moment please...
        </h2>
        <p className="text-gray-500">
          We're running some final checks to set up your account.
        </p>
      </div>
    </>
  );
}
