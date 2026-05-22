import type { Component } from 'vue';
import NpcConfig from '~/components/builder/NpcConfig.vue';
import { createDefaultRegistry } from '~/gameplay';
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

export interface StoryForm {
  storyId: string;
  title: string;
  genre: string;
  coverArt: string;
  description: string;
}

interface SavedStory {
  id: string;
  authorName: string;
  storyId: string;
  title: string;
}

export function useStoryBuilder() {
  const DEFAULT_MODULE_TYPES = ['npc', 'plan', 'lore'];

  const registry = createDefaultRegistry();
  const registryModules = computed(() =>
    Object.values(registry.getAll()).map(m => ({ type: m.type })),
  );

  const form = reactive<StoryForm>({
    storyId: '',
    title: '',
    genre: '',
    coverArt: '',
    description: '',
  });

  const activeModuleTypes = ref<Set<string>>(new Set(DEFAULT_MODULE_TYPES));
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
  const result = ref<SavedStory | null>(null);

  const isDirty = computed(() => {
    if (
      form.storyId !== (initialForm.value.storyId ?? '') ||
      form.title !== (initialForm.value.title ?? '') ||
      form.genre !== (initialForm.value.genre ?? '') ||
      form.coverArt !== (initialForm.value.coverArt ?? '') ||
      form.description !== (initialForm.value.description ?? '')
    ) return true;

    const initialKeys = Object.keys(initialModulesConfig.value).sort();
    const currentKeys = Object.keys(modulesConfig.value).sort();
    if (initialKeys.length !== currentKeys.length) return true;
    if (initialKeys.some((k, i) => k !== currentKeys[i])) return true;
    return initialKeys.some(
      (k) => JSON.stringify(modulesConfig.value[k]) !== JSON.stringify(initialModulesConfig.value[k]),
    );
  });

  const hasDraft = ref(false);

  const draftErrors = computed(() => {
    const errors: string[] = [];
    if (!form.storyId.trim()) errors.push('Story ID is required');
    return errors;
  });

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

  const moduleComponents: Record<string, unknown> = {
    'npc': NpcConfig,
  };

  const availableModules = computed(() =>
    registryModules.value.filter((m) => !activeModuleTypes.value.has(m.type)),
  );

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

  const modulesJson = computed(() => JSON.stringify(modulesConfig.value, null, 2));

  function populateFrom(story: {
    id?: string;
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

    initialForm.value = { ...form };

    const mods = (story.modules ?? {}) as Record<string, unknown>;
    initialModulesConfig.value = { ...mods };
    modulesConfig.value = JSON.parse(JSON.stringify(mods));
    activeModuleTypes.value = new Set(Object.keys(mods));
    hasDraft.value = true;

    if (story.id) {
      result.value = { id: story.id!, authorName: '', storyId: story.storyId, title: story.title };
    }
  }

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
      const payload = buildPayload();
      const id = result.value?.id ?? crypto.randomUUID();
      const now = new Date().toISOString();

      const existing = await unwrap(commands.storyGet(id));

      if (existing) {
        await unwrap(commands.storySave({
          version: 1,
          id,
          title: payload.title ?? 'Untitled',
          description: payload.description ?? null,
          config: JSON.stringify(payload.modules),
          created_at: now,
          updated_at: now,
        }));
      } else {
        await unwrap(commands.storySave({
          version: 1,
          id,
          title: payload.title ?? 'Untitled',
          description: payload.description ?? null,
          config: JSON.stringify(payload.modules),
          created_at: now,
          updated_at: now,
        }));
      }

      const saved: SavedStory = {
        id,
        authorName: '',
        storyId: payload.storyId ?? 'story',
        title: payload.title ?? 'Untitled',
      };
      result.value = saved;
      hasDraft.value = true;
      initialForm.value = { ...form };
      initialModulesConfig.value = JSON.parse(JSON.stringify(modulesConfig.value));
      return saved;
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to save draft';
      alert(msg);
    } finally {
      saving.value = false;
    }
  }

  async function publish() {
    if (!canPublish.value) return;
    submitting.value = true;
    result.value = null;
    try {
      const saved = await saveDraft();
      result.value = saved ?? null;
      return result.value;
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to publish';
      alert(msg);
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
