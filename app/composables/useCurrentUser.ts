export interface UserShape {
  id: string;
  name: string;
  email: string;
  emailVerified: boolean;
  image: string | null;
  isAuthor: boolean;
}

/** Shared reactive state. Fetched once, refreshed on auth changes. */
const currentUser = ref<UserShape | null>(null);
const sessions = ref<Array<{
  id: number;
  story: { id: number; title: string };
}>>([]);

let initialized = false;

export function useCurrentUser() {
  async function refresh() {
    const [me, sess] = await Promise.all([
      $fetch<{ user: UserShape | null }>('/api/user/me'),
      $fetch<{ sessions: Array<{ id: number; story: { id: number; title: string } }> }>('/api/sessions'),
    ]);
    currentUser.value = me.user;
    sessions.value = sess.sessions;
  }

  if (!initialized) {
    initialized = true;
    refresh();
  }

  return { currentUser, sessions, refresh };
}
