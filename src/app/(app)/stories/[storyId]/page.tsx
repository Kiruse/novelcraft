"use client";

import { useSignals } from '@preact/signals-react/runtime';
import { useComputedQuery } from 'convex-signals-client/react';
import { useParams } from 'next/navigation';
import { convex } from '~/config';
import { api } from '~convex/_generated/api';
import { Id } from '~convex/_generated/dataModel';

export default function StoryPage() {
  useSignals();
  const { storyId } = useParams();

  const story = useComputedQuery(convex, api.stories.get, () => [{ id: storyId as Id<'stories'> }]);

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <h1>{story.value?.name ?? 'Loading Story...'}</h1>
    </div>
  );
}
