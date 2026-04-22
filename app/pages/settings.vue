<template>
  <div class="settings-page">
    <h1 class="page-title">Settings</h1>

    <section class="card">
      <h2 class="card-title">Profile</h2>
      <table class="profile-table">
        <tr class="profile-row">
          <th class="profile-label">Name</th>
          <td class="profile-value">{{ currentUser?.name }}</td>
        </tr>
        <tr class="profile-row">
          <th class="profile-label">Email</th>
          <td class="profile-value">{{ currentUser?.email }}</td>
        </tr>
        <tr class="profile-row">
          <th class="profile-label">Email verified</th>
          <td class="profile-value">
            <span :class="currentUser?.emailVerified ? 'badge badge--success' : 'badge badge--warning'">
              {{ currentUser?.emailVerified ? 'Yes' : 'No' }}
            </span>
          </td>
        </tr>
        <tr class="profile-row">
          <th class="profile-label">Story Author</th>
          <td class="profile-value">
            <span :class="currentUser?.isAuthor ? 'badge badge--success' : 'badge badge--muted'">
              {{ currentUser?.isAuthor ? 'Enabled' : 'Not enabled' }}
            </span>
          </td>
        </tr>
      </table>
    </section>

    <section v-if="!currentUser?.isAuthor" class="card">
      <h2 class="card-title">Become a Story Author</h2>
      <p class="card-desc">Enter your registration code to unlock the story builder.</p>
      <form class="redeem-form" @submit.prevent="redeemCode">
        <input
          v-model="authorCode"
          type="text"
          class="input"
          placeholder="Enter registration code"
        />
        <button type="submit" class="btn btn--primary" :disabled="redeeming">
          {{ redeeming ? 'Redeeming...' : 'Redeem' }}
        </button>
      </form>
      <p v-if="redeemError" class="error">{{ redeemError }}</p>
      <p v-if="redeemSuccess" class="success">{{ redeemSuccess }}</p>
    </section>

    <section class="card">
      <h2 class="card-title">Account</h2>
      <button class="btn btn--danger" @click="signOut">
        Sign out
      </button>
    </section>
  </div>
</template>

<script setup lang="ts">
import { authClient } from '~/composables/useAuthClient';
import { useCurrentUser } from '~/composables/useCurrentUser';

const { currentUser, refresh: refreshAuth } = useCurrentUser();
const refreshMe = refreshAuth;

const authorCode = ref('');
const redeeming = ref(false);
const redeemError = ref('');
const redeemSuccess = ref('');

async function redeemCode() {
  redeemError.value = '';
  redeemSuccess.value = '';
  redeeming.value = true;

  try {
    const res = await $fetch('/api/user/redeem-author', {
      method: 'POST',
      body: { code: authorCode.value },
    });
    if (res.isAuthor) {
      redeemSuccess.value = 'Story Author mode enabled!';
      authorCode.value = '';
      await refreshAuth();
    }
  } catch (e: unknown) {
    redeemError.value =
      (e as { data?: { statusMessage?: string } })?.data?.statusMessage ?? 'Failed to redeem code';
  } finally {
    redeeming.value = false;
  }
}

async function signOut() {
  await authClient.signOut();
  await refreshAuth();
  navigateTo('/auth/login');
}
</script>

<style scoped>
.settings-page {
  max-inline-size: var(--size-lg);
  margin-inline: auto;
  padding: var(--size-8);
  display: flex;
  flex-direction: column;
  gap: var(--size-6);
}

.page-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
}

.card {
  background: var(--surface-1);
  border-radius: var(--radius-4);
  padding: var(--size-6);
  box-shadow: var(--shadow-1);
}

.card-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-4);
}

.card-desc {
  font-size: var(--font-size-2);
  color: var(--text-2);
  margin-block-end: var(--size-4);
}

.profile-table {
  inline-size: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-2);
}

.profile-row {
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
}

.profile-row:hover {
  background: var(--surface-3);
}

.profile-row:hover .profile-label {
  color: var(--text-1);
}

.profile-label {
  text-align: start;
  font-weight: var(--font-weight-5);
  color: var(--text-2);
  padding: var(--size-3) var(--size-4);
  white-space: nowrap;
  inline-size: 30%;
}

.profile-value {
  padding: var(--size-3) var(--size-4);
  color: var(--text-1);
}

.badge {
  font-size: var(--font-size-0);
  padding: var(--size-1) var(--size-2);
  border-radius: var(--radius-round);
  font-weight: var(--font-weight-5);
}

.badge--success {
  background: var(--green-2);
  color: var(--green-9);
}

.badge--warning {
  background: var(--orange-2);
  color: var(--orange-9);
}

.badge--muted {
  background: var(--gray-2);
  color: var(--gray-7);
}

.redeem-form {
  display: flex;
  gap: var(--size-3);
}

.input {
  flex: 1;
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

.btn {
  padding: var(--size-3) var(--size-6);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
}

.btn--primary {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.btn--primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn--danger {
  background: var(--red-2);
  color: var(--red-9);
}

.btn--danger:hover {
  background: var(--red-3);
}

.error {
  color: var(--red-6);
  font-size: var(--font-size-1);
  margin-block-start: var(--size-2);
}

.success {
  color: var(--green-6);
  font-size: var(--font-size-1);
  margin-block-start: var(--size-2);
}
</style>
