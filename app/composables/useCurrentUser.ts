export interface UserShape {
  id: string;
  name: string;
  email: string;
  emailVerified: boolean;
  image: string | null;
  isAuthor: boolean;
}

export interface AuthorStoryShape {
  id: number;
  storyId: string;
  title: string;
  version: number;
  genre: string | null;
  coverArt: string | null;
}

const currentUser = ref<UserShape | null>(null);
const authorStories = ref<AuthorStoryShape[]>([]);

let initialized = false;

export function useCurrentUser() {
  async function refresh() {
    const [me, stories] = await Promise.all([
      $fetch<{ user: UserShape | null }>('/api/user/me').catch(() => ({ user: null as UserShape | null })),
      $fetch<{ stories: AuthorStoryShape[] }>('/api/user/stories').catch(() => ({ stories: [] as AuthorStoryShape[] })),
    ]);
    currentUser.value = me.user;
    authorStories.value = stories.stories;
  }

  if (!initialized) {
    initialized = true;
    refresh();
  }

  return { currentUser, authorStories, refresh };
}
