<template>
  <div v-if="error" class="not-found">
    <h1>Story not found</h1>
    <p>{{ error.statusMessage }}</p>
    <NuxtLink to="/builder" class="back-link">Back to Builder</NuxtLink>
  </div>
  <div v-else-if="canBuild" class="builder-page">
    <BuilderForm
      ref="formRef"
      story-id-disabled
      :initial-modules-config="initialModulesConfig"
    >
      <template #header>
        <nav class="edit-breadcrumb">
          <NuxtLink to="/builder" class="breadcrumb-link">Story Builder</NuxtLink>
          <span class="breadcrumb-sep">/</span>
          <span class="breadcrumb-current">Edit: {{ formRef?.form?.title || 'Untitled' }}</span>
        </nav>
      </template>

      <template #result="{ result }">
        <p>✅ Published as <NuxtLink :to="`/stories/${result.authorName}/${result.storyId}`">{{ result.title }}</NuxtLink></p>
      </template>
    </BuilderForm>
  </div>
  <div v-else class="disabled">
    <p>Story Builder requires author access. <NuxtLink to="/settings">Enable it in Settings</NuxtLink></p>
  </div>
</template>

<script setup lang="ts">
const { currentUser } = useCurrentUser();
const canBuild = computed(() => currentUser.value?.isAuthor === true);
const route = useRoute();
const author = route.params.author as string;
const storyId = route.params.id as string;

const formRef = ref<{ populateFrom: (s: any) => void; form: any; isDirty: boolean; hasDraft: boolean; result: any } | null>(null);

// Load draft (or create one from latest version)
const { data: draftData, error } = await useFetch<{
  story: {
    id: number;
    storyId: string;
    title: string;
    genre: string | null;
    coverArt: string | null;
    description: string | null;
    modules: unknown;
  };
}>(`/api/stories/${author}/${storyId}/draft`);

// Extract initial modules config to pass as prop (needed before component mounts)
const initialModulesConfig = computed(() => {
  const mods = draftData.value?.story?.modules;
  return mods ? (mods as Record<string, unknown>) : undefined;
});

// Populate form once component is mounted and data is available
onMounted(() => {
  if (draftData.value?.story && formRef.value) {
    formRef.value.populateFrom(draftData.value.story);
  }
});
</script>

<style scoped>
.builder-page {
  max-inline-size: var(--size-content-3);
  margin-inline: auto;
  padding: var(--size-8);
}

.edit-breadcrumb {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  font-size: var(--font-size-1);
  margin-block-end: var(--size-6);
  color: var(--text-2);
}

.breadcrumb-link {
  color: var(--text-2);
  text-decoration: none;
}

.breadcrumb-link:hover {
  color: var(--text-1);
}

.breadcrumb-sep {
  color: var(--surface-4);
}

.breadcrumb-current {
  font-weight: var(--font-weight-5);
}

.disabled {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}

.not-found {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}

.not-found h1 {
  font-size: var(--font-size-5);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-3);
}

.not-found p {
  margin-block-end: var(--size-4);
}

.back-link {
  color: var(--indigo-6);
  text-decoration: none;
}

.back-link:hover {
  text-decoration: underline;
}
</style>
