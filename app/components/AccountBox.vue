<template>
  <div class="account-box">
    <div v-if="!user" class="account-guest">
      <NuxtLink
        v-if="expanded"
        to="/auth/login"
        class="auth-btn"
        @click="$emit('closeDrawer')"
      >
        Log in
      </NuxtLink>
      <NuxtLink
        v-if="expanded"
        to="/auth/login?signup=1"
        class="auth-btn auth-btn--cta"
        @click="$emit('closeDrawer')"
      >
        Sign up
      </NuxtLink>
    </div>

    <div v-else>
      <button class="account-trigger" @click="menuOpen = !menuOpen">
        <img v-if="user.image" :src="user.image" :alt="user.name" class="avatar" />
        <div v-else class="avatar avatar--fallback">{{ initials }}</div>
        <span v-if="expanded" class="account-info">
          <span class="account-name">{{ user.name }}</span>
          <span v-if="activeProfile" class="account-profile">{{ activeProfile.name }}</span>
          <span v-else class="account-profile account-profile--muted">- default profile -</span>
        </span>
        <Chevron v-if="expanded" :open="menuOpen" direction="down" />
      </button>

      <Transition name="slide-up">
        <div v-if="menuOpen" class="account-menu">
          <button class="account-menu-item" @click="profilesOpen = true">
            <UserOutlined class="account-menu-icon" /> Profiles
          </button>
          <button class="account-menu-item" @click="menuOpen = false; $emit('closeDrawer'); openShortcuts()">
            <QuestionCircleOutlined class="account-menu-icon" /> Shortcuts
          </button>
          <NuxtLink to="/settings" class="account-menu-item" @click="menuOpen = false; $emit('closeDrawer')">
            <SettingOutlined class="account-menu-icon" /> Settings
          </NuxtLink>
          <button class="account-menu-item account-menu-item--danger" @click="signOut">
            <LogoutOutlined class="account-menu-icon" /> Sign out
          </button>
        </div>
      </Transition>
    </div>

    <ProfilesDialog
      :open="profilesOpen"
      :profiles="profileStore.profiles.value ?? []"
      :active-profile="activeProfile"
      :max-profiles="profileStore.maxProfiles"
      :default-fields="profileStore.defaultFields"
      @close="profilesOpen = false"
      @create="onCreateProfile"
      @update="onUpdateProfile"
      @remove="onRemoveProfile"
      @set-active="onSetActive"
    />
  </div>
</template>

<script setup lang="ts">
import type { UserShape } from '~/composables/useCurrentUser';
import { useCurrentUser } from '~/composables/useCurrentUser';
import { authClient } from '~/composables/useAuthClient';
import { useProfiles } from '~/composables/useProfiles';
import { SettingOutlined, LogoutOutlined, UserOutlined, QuestionCircleOutlined } from '@ant-design/icons-vue';

const props = defineProps<{
  user: UserShape | null;
  expanded: boolean;
}>();

defineEmits<{
  closeDrawer: [];
}>();

const menuOpen = ref(false);
const profilesOpen = ref(false);
const profileStore = useProfiles();
const { activeProfile } = profileStore;
const { show: openShortcuts } = useShortcutsDialog();

const initials = computed(() => {
  if (!props.user?.name) return '?';
  return props.user.name
    .split(/\s+/)
    .map(w => w[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();
});

async function signOut() {
  menuOpen.value = false;
  await authClient.signOut();
  await useCurrentUser().refresh();
  navigateTo('/auth/login');
}

async function onCreateProfile(name: string, fields: Record<string, string>) {
  const profile = await profileStore.create(name, fields);
  if (profile) await profileStore.setActive(profile.id);
}

async function onUpdateProfile(id: string, patch: { name?: string; fields?: Record<string, string> }) {
  await profileStore.update(id, patch);
}

async function onRemoveProfile(id: string) {
  await profileStore.remove(id);
}

async function onSetActive(id: string) {
  await profileStore.setActive(id);
}
</script>

<style scoped>
.account-box {
  border-block-start: var(--border-size-1) solid var(--surface-3);
  padding: var(--size-3);
}

.account-guest {
  display: flex;
  gap: var(--size-2);
}

.auth-btn {
  flex: 1;
  text-align: center;
  padding: var(--size-2) var(--size-3);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
  text-decoration: none;
  color: var(--text-2);
}

.auth-btn:hover {
  background: var(--surface-3);
  color: var(--text-1);
}

.auth-btn--cta {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.auth-btn--cta:hover {
  color: var(--gray-0);
  opacity: 0.9;
}

.account-trigger {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  background: none;
  border: none;
  cursor: pointer;
  padding: var(--size-2);
  border-radius: var(--radius-2);
  inline-size: 100%;
  text-align: start;
}

.account-trigger:hover {
  background: var(--surface-3);
}

.avatar {
  inline-size: var(--size-7);
  block-size: var(--size-7);
  border-radius: var(--radius-round);
  object-fit: cover;
  flex-shrink: 0;
  background: var(--surface-3);
}

.avatar--fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-6);
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.account-info {
  flex: 1;
  min-inline-size: 0;
  display: flex;
  flex-direction: column;
}

.account-name {
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-profile {
  font-size: var(--font-size-0);
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-profile--muted {
  font-style: italic;
  opacity: 0.6;
}

.account-menu {
  display: flex;
  flex-direction: column;
  padding: var(--size-1) 0;
}

.account-menu-item {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  padding: var(--size-2) var(--size-3);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
  text-decoration: none;
  color: var(--text-1);
  background: none;
  border: none;
  cursor: pointer;
  inline-size: 100%;
  text-align: start;
}

.account-menu-item:hover {
  background: var(--surface-3);
}

.account-menu-item--danger {
  color: var(--red-7);
}

.account-menu-item--danger:hover {
  background: var(--red-2);
}

.account-menu-icon {
  font-size: var(--font-size-2);
  line-height: 1;
}

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all var(--animation-duration, 0.15s) var(--ease-2);
}

.slide-up-enter-from,
.slide-up-leave-to {
  opacity: 0;
  transform: translateY(var(--size-2));
}
</style>
