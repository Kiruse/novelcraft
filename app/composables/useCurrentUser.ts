export interface UserShape {
  id: string;
  name: string;
  email: string;
  emailVerified: boolean;
  image: string | null;
  isAuthor: boolean;
}

export interface SessionShape {
  id: number;
  story: {
    id: number;
    storyId: string;
    title: string;
    version: number;
    author: { name: string };
  };
}

export interface AuthorStoryShape {
  id: number;
  storyId: string;
  title: string;
  version: number;
  genre: string | null;
  coverArt: string | null;
}

/** Shared reactive state. Fetched once, refreshed on auth changes. */
const currentUser = ref<UserShape | null>(null);
const sessions = ref<SessionShape[]>([]);
const authorStories = ref<AuthorStoryShape[]>([]);

let initialized = false;

export function useCurrentUser() {
  async function refresh() {
    const [me, sess, stories] = await Promise.all([
      $fetch<{ user: UserShape | null }>('/api/user/me').catch(() => ({ user: null as UserShape | null })),
      $fetch<{ sessions: SessionShape[] }>('/api/sessions').catch(() => ({ sessions: [] as SessionShape[] })),
      $fetch<{ stories: AuthorStoryShape[] }>('/api/user/stories').catch(() => ({ stories: [] as AuthorStoryShape[] })),
    ]);
    currentUser.value = me.user;
    sessions.value = sess.sessions;
    authorStories.value = stories.stories;
  }

  if (!initialized) {
    initialized = true;
    refresh();
  }

  return { currentUser, sessions, authorStories, refresh };
}
