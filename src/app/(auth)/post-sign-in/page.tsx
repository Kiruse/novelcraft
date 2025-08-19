'use client';

import { useSignalEffect } from '@preact/signals-react';
import { useRouter } from 'next/navigation';
import { Spinner } from '~/components/Spinner';
import { convex } from '~/config';
import { api } from '~convex/_generated/api';

export default function PostSignInPage() {
  return (
    <>
      <div className="text-center">
        <h2 className="text-xl font-semibold text-gray-200 mb-2">
          Authentication Complete
        </h2>
        <p className="text-gray-500 mb-6">
          This page will handle post-authentication flow when the new auth system is implemented.
        </p>
        <a
          href="/dashboard"
          className="inline-block bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-lg transition-colors duration-200"
        >
          Go to Dashboard
        </a>
      </div>
    </>
  );
}
