import MapGraphConfig from '~/components/builder/MapGraphConfig.vue';
import NpcConfig from '~/components/builder/NpcConfig.vue';
import EventConfig from '~/components/builder/EventConfig.vue';

export interface StoryForm {
  storyId: string;
  title: string;
  genre: string;
  coverArt: string;
  description: string;
}

export function useStoryBuilder() {
  const { data: modulesData } = useFetch('/api/modules');
  const registryModules = computed(() => modulesData.value?.modules ?? []);

  // --- Form state ---

  const form = reactive<StoryForm>({
    storyId: '',
    title: '',
    genre: '',
    coverArt: '',
    description: '',
  });

  const activeModuleTypes = ref<Set<string>>(new Set());
  const modulesConfig = ref<Record<string, unknown>>({});
  const initialModulesConfig = ref<Record<string, unknown>>({});
  const initialForm = ref<StoryForm>({
    storyId: '',
    title: '',
    genre: '',
    coverArt: '',
    description: '',
  });
  const moduleDialogEl = ref<HTMLDialogElement | null>(null);

  const submitting = ref(false);
  const saving = ref(false);
  const result = ref<{ id: number; title: string } | null>(null);

  // --- Dirty tracking ---

  const isDirty = computed(() => {
    // Form fields
    if (
      form.storyId !== (initialForm.value.storyId ?? '') ||
      form.title !== (initialForm.value.title ?? '') ||
      form.genre !== (initialForm.value.genre ?? '') ||
      form.coverArt !== (initialForm.value.coverArt ?? '') ||
      form.description !== (initialForm.value.description ?? '')
    ) return true;

    // Module config changes
    const initialKeys = Object.keys(initialModulesConfig.value).sort();
    const currentKeys = Object.keys(modulesConfig.value).sort();
    if (initialKeys.length !== currentKeys.length) return true;
    if (initialKeys.some((k, i) => k !== currentKeys[i])) return true;
    return initialKeys.some(
      (k) => JSON.stringify(modulesConfig.value[k]) !== JSON.stringify(initialModulesConfig.value[k]),
    );
  });

  const hasDraft = ref(false);

  // --- Validation ---

  /** Draft requires only storyId (+ title for new drafts). */
  const draftErrors = computed(() => {
    const errors: string[] = [];
    if (!form.storyId.trim()) errors.push('Story ID is required');
    return errors;
  });

  /** Publishing requires storyId, title, genre, and description ≥ 100 chars. */
  const publishErrors = computed(() => {
    const errors: string[] = [];
    if (!form.storyId.trim()) errors.push('Story ID is required');
    if (!form.title.trim()) errors.push('Title is required');
    if (!form.genre.trim()) errors.push('Genre is required');
    if (!form.description.trim()) {
      errors.push('Description is required');
    } else if (form.description.trim().length < 100) {
      errors.push(`Description must be at least 100 characters (${form.description.trim().length}/100)`);
    }
    return errors;
  });

  const canSaveDraft = computed(() => draftErrors.value.length === 0);
  const canPublish = computed(() => publishErrors.value.length === 0);

  // --- Module management ---

  const moduleComponents: Record<string, unknown> = {
    'map::graph': MapGraphConfig,
    'npc': NpcConfig,
    'event': EventConfig,
  };

  const availableModules = computed(() =>
    registryModules.value.filter((m) => !activeModuleTypes.value.has(m.type)),
  );

  /** Resolve active module types to their registered config UI components. */
  const activeConfigComponents = computed(() => {
    return [...activeModuleTypes.value]
      .map((type) => {
        const comp = moduleComponents[type];
        return comp ? [type, comp] as const : undefined;
      })
      .filter(Boolean) as [string, Component][];
  });

  function addModule(type: string) {
    activeModuleTypes.value = new Set([...activeModuleTypes.value, type]);
  }

  function removeModule(type: string) {
    const next = new Set(activeModuleTypes.value);
    next.delete(type);
    activeModuleTypes.value = next;
    delete modulesConfig.value[type];
  }

  function setModuleConfig(type: string, value: unknown) {
    if (value === undefined) {
      delete modulesConfig.value[type];
    } else {
      modulesConfig.value[type] = value;
    }
  }

  function openModuleDialog() {
    moduleDialogEl.value?.showModal();
  }

  function closeModuleDialog() {
    moduleDialogEl.value?.close();
  }

  // --- Serialize (generic — no module-specific knowledge) ---

  const modulesJson = computed(() => JSON.stringify(modulesConfig.value, null, 2));

  // --- Populate from draft/published data ---

  function populateFrom(story: {
    id?: number;
    storyId: string;
    title: string;
    genre: string | null;
    coverArt: string | null;
    description: string | null;
    modules: unknown;
  }) {
    form.storyId = story.storyId;
    form.title = story.title;
    form.genre = story.genre ?? '';
    form.coverArt = story.coverArt ?? '';
    form.description = story.description ?? '';

    // Track initial form state for dirty detection
    initialForm.value = { ...form };

    const mods = (story.modules ?? {}) as Record<string, unknown>;
    initialModulesConfig.value = { ...mods };
    modulesConfig.value = JSON.parse(JSON.stringify(mods));
    activeModuleTypes.value = new Set(Object.keys(mods));
    hasDraft.value = true;

    // Set result so Test button can navigate
    if (story.id) {
      result.value = { id: story.id, title: story.title };
    }
  }

  // --- Actions ---

  function buildPayload() {
    return {
      storyId: form.storyId,
      title: form.title || undefined,
      genre: form.genre || undefined,
      coverArt: form.coverArt || undefined,
      description: form.description || undefined,
      modules: { ...modulesConfig.value },
    };
  }

  async function saveDraft() {
    if (!canSaveDraft.value) return;
    saving.value = true;
    try {
      const res: any = await $fetch('/api/stories/draft', {
        method: 'PUT',
        body: buildPayload(),
      });
      result.value = res.story;
      hasDraft.value = true;
      // Update initial state so dirty resets
      initialForm.value = { ...form };
      initialModulesConfig.value = JSON.parse(JSON.stringify(modulesConfig.value));
      return res.story;
    } catch (e: any) {
      alert(e?.data?.statusMessage ?? e?.message ?? 'Failed to save draft');
    } finally {
      saving.value = false;
    }
  }

  async function publish() {
    if (!canPublish.value) return;
    submitting.value = true;
    result.value = null;
    try {
      const res: any = await $fetch('/api/stories/publish', {
        method: 'POST',
        body: buildPayload(),
      });
      result.value = res.story;
      return res.story;
    } catch (e: any) {
      alert(e?.data?.statusMessage ?? e?.message ?? 'Failed to publish');
    } finally {
      submitting.value = false;
    }
  }

  return {
    form,
    activeModuleTypes,
    modulesConfig,
    initialModulesConfig,
    moduleDialogEl,
    submitting,
    saving,
    result,
    isDirty,
    hasDraft,
    draftErrors,
    publishErrors,
    canSaveDraft,
    canPublish,
    availableModules,
    activeConfigComponents,
    modulesJson,
    addModule,
    removeModule,
    setModuleConfig,
    openModuleDialog,
    closeModuleDialog,
    populateFrom,
    saveDraft,
    publish,
  };
}
