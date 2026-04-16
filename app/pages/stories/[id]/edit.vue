<template>
  <div v-if="enabled" class="builder-page">
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
        <p>✅ Published as <NuxtLink :to="`/stories/${result.id}`">{{ result.title }}</NuxtLink></p>
      </template>
    </BuilderForm>
  </div>
  <div v-else class="disabled">
    <p>Story Builder is not enabled.</p>
  </div>
</template>

<script setup lang="ts">
const { storyBuilder: enabled } = useRuntimeConfig().public;
const route = useRoute();
const id = route.params.id as string;

const formRef = ref<{ populateFrom: (s: any) => void; form: any } | null>(null);

// Load draft (or create one from latest version)
const { data: draftData } = await useFetch<{
  story: {
    storyId: string;
    title: string;
    genre: string | null;
    coverArt: string | null;
    description: string | null;
    modules: unknown;
  };
}>(`/api/stories/${id}/draft`);

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
</style>
