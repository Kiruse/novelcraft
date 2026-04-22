<template>
  <div class="auth-page">
    <div class="auth-card">
      <h1 class="auth-title">NovelCraft</h1>

      <!-- Magic link sent confirmation -->
      <template v-if="magicLinkSent">
        <p class="auth-subtitle">Check your email</p>
        <div class="sent-confirmation">
          <p class="sent-text">
            We sent a sign-in link to <strong>{{ email }}</strong>. It'll expire in 5 minutes.
          </p>
          <a
            v-if="emailDomain"
            :href="emailDomain.href"
            target="_blank"
            rel="noopener"
            class="inbox-link"
          >
            Open {{ emailDomain.label }} ↗
          </a>
          <button class="toggle-btn" @click="magicLinkSent = false">
            Use a different email
          </button>
        </div>
      </template>

      <!-- Normal form -->
      <template v-else>
        <p class="auth-subtitle">{{ isSignUp ? 'Create your account' : 'Welcome back' }}</p>

        <form class="auth-form" @submit.prevent="submit">
          <div v-if="isSignUp" class="field">
            <label for="name" class="label">Name</label>
            <input
              id="name"
              v-model="name"
              type="text"
              class="input"
              placeholder="Your name"
              autocomplete="name"
              required
            />
          </div>

          <div class="field">
            <label for="email" class="label">Email</label>
            <input
              id="email"
              v-model="email"
              type="email"
              class="input"
              placeholder="you@example.com"
              autocomplete="email"
              required
            />
          </div>

          <div class="field">
            <label for="password" class="label">Password</label>
            <input
              id="password"
              v-model="password"
              type="password"
              class="input input--password"
              :class="{ 'input--password-active': password.length > 0 }"
              placeholder="Leave empty for a magic link"
              :autocomplete="isSignUp ? 'new-password' : 'current-password'"
            />
          </div>

          <button type="submit" class="submit-btn" :disabled="loading">
            {{ loading ? 'Please wait...' : submitLabel }}
          </button>

          <p v-if="error" class="error">{{ error }}</p>
        </form>

        <div class="auth-toggles">
          <button class="toggle-btn" @click="isSignUp = !isSignUp">
            {{ isSignUp ? 'Already have an account? Sign in' : "Don't have an account? Sign up" }}
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { authClient } from '~/composables/useAuthClient';
import { useCurrentUser } from '~/composables/useCurrentUser';

definePageMeta({ layout: false });

const route = useRoute();
const isSignUp = ref(route.query.signup !== undefined);
const magicLinkSent = ref(false);
const name = ref('');
const email = ref('');
const password = ref('');
const loading = ref(false);
const error = ref('');

const KNOWN_DOMAINS: Record<string, string> = {
  'gmail.com': 'https://mail.google.com',
  'googlemail.com': 'https://mail.google.com',
  'outlook.com': 'https://outlook.live.com',
  'hotmail.com': 'https://outlook.live.com',
  'yahoo.com': 'https://mail.yahoo.com',
  'proton.me': 'https://mail.proton.me',
  'pm.me': 'https://mail.proton.me',
  'icloud.com': 'https://www.icloud.com/mail',
  'aol.com': 'https://mail.aol.com',
};

const emailDomain = computed(() => {
  const domain = email.value.split('@')[1]?.toLowerCase();
  if (!domain) return null;
  const href = KNOWN_DOMAINS[domain] ?? `https://${domain}`;
  const label = KNOWN_DOMAINS[domain] ? domain : domain;
  return { href, label };
});

const submitLabel = computed(() => {
  if (password.value.length > 0) return isSignUp.value ? 'Create account' : 'Sign in';
  return isSignUp.value ? 'Create account with magic link' : 'Send magic link';
});

async function submit() {
  error.value = '';
  loading.value = true;

  try {
    if (password.value.length > 0) {
      if (isSignUp.value) {
        const res = await authClient.signUp.email({
          name: name.value,
          email: email.value,
          password: password.value,
          callbackURL: '/',
        });
        if (res.error) {
          error.value = res.error.message ?? 'Sign up failed';
        } else {
          magicLinkSent.value = true;
        }
      } else {
        const res = await authClient.signIn.email({
          email: email.value,
          password: password.value,
        });
        if (res.error) {
          error.value = res.error.message ?? 'Sign in failed';
        } else {
          await useCurrentUser().refresh();
          navigateTo('/');
        }
      }
    } else {
      const res = await authClient.signIn.magicLink({
        email: email.value,
        ...(isSignUp.value ? { name: name.value } : {}),
        callbackURL: '/',
      });
      if (res.error) {
        error.value = res.error.message ?? 'Magic link failed';
      } else {
        magicLinkSent.value = true;
      }
    }
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : 'Something went wrong';
  } finally {
    loading.value = false;
  }
}
</script>

<style scoped>
.auth-page {
  min-block-size: 100dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--size-6);
}

.auth-card {
  inline-size: 100%;
  max-inline-size: var(--size-sm);
  background: var(--surface-1);
  border-radius: var(--radius-4);
  box-shadow: var(--shadow-3);
  padding: var(--size-8);
}

.auth-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-9);
  text-align: center;
  margin-block-end: var(--size-1);
}

.auth-subtitle {
  text-align: center;
  color: var(--text-2);
  font-size: var(--font-size-2);
  margin-block-end: var(--size-6);
}

/* --- Sent confirmation --- */

.sent-confirmation {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--size-5);
  text-align: center;
}

.sent-text {
  font-size: var(--font-size-2);
  color: var(--text-2);
  line-height: var(--font-lineheight-4);
}

.inbox-link {
  display: inline-flex;
  align-items: center;
  gap: var(--size-2);
  background: var(--brand-gradient);
  color: var(--gray-0);
  padding: var(--size-3) var(--size-6);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  text-decoration: none;
}

.inbox-link:hover {
  opacity: 0.9;
}

/* --- Form --- */

.auth-form {
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
}

.label {
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
}

.input {
  padding: var(--size-3);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  background: var(--surface-2);
  color: var(--text-1);
  font-size: var(--font-size-2);
}

.input:focus {
  outline: none;
  border-color: var(--indigo-6);
  box-shadow: 0 0 0 var(--border-size-2) var(--indigo-2);
}

.input--password {
  opacity: 0.7;
  transition: opacity var(--animation-duration, 0.15s) var(--ease-2);
}

.input--password:focus,
.input--password-active {
  opacity: 1;
}

.submit-btn {
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  padding: var(--size-3) var(--size-6);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  margin-block-start: var(--size-2);
}

.submit-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error {
  color: var(--red-6);
  font-size: var(--font-size-1);
  text-align: center;
}

.auth-toggles {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
  margin-block-start: var(--size-5);
  align-items: center;
}

.toggle-btn {
  background: none;
  border: none;
  color: var(--indigo-6);
  font-size: var(--font-size-1);
  cursor: pointer;
}

.toggle-btn:hover {
  text-decoration: underline;
}
</style>
