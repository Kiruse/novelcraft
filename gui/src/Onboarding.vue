<template>
  <div class="onboarding">
    <div class="onboarding-container">
      <div class="onboarding-step" :class="{ 'onboarding-step--enter': enterActive }">
        <Transition name="fade" mode="out-in">
          <div v-if="step === 0" key="welcome" class="step-content">
            <h1 class="step-title">Welcome to NovelCraft</h1>
            <p class="step-text">
              An interactive fiction engine that weaves dynamic narratives using
              LLMs. Create immersive stories where your choices
              shape the world, characters react to your decisions, and every
              playthrough tells a different tale.
            </p>
            <p class="step-disclaimer">
              This is an early alpha build. Features may change, and you may
              encounter bugs. Your feedback helps shape the future of NovelCraft.
            </p>
          </div>

          <div v-else-if="step === 1" key="models" class="step-content">
            <h2 class="step-title">Configure your models</h2>
            <p class="step-text">
              NovelCraft needs at least one LLM model to generate stories.
              Configure your model hosts below. You can always change these
              later in Settings.
            </p>
            <ModelsConfigurator />
          </div>
        </Transition>
      </div>

      <div class="onboarding-footer">
        <div class="step-indicators">
          <span
            v-for="(_, i) in STEPS"
            :key="i"
            class="step-dot"
            :class="{ 'step-dot--active': i === step }"
          />
        </div>

        <div class="footer-actions">
          <button v-if="step > 0" class="btn btn--ghost" @click="step--">Back</button>
          <button v-if="step < lastStep" class="btn btn--primary" @click="step++">Next</button>
          <button v-else class="btn btn--primary" @click="finish">Get Started</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import ModelsConfigurator from '~/components/ModelsConfigurator.vue';
import { useOnboarding } from '~/composables/useOnboarding';

const STEPS = [null, null] as const;
const lastStep = STEPS.length - 1;

const step = ref(0);
const enterActive = ref(false);

const { complete } = useOnboarding();

function finish() {
  complete();
}

watch(step, () => {
  enterActive.value = true;
  setTimeout(() => (enterActive.value = false), 300);
});
</script>

<style scoped>
.onboarding {
  block-size: 100dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--size-6);
  overflow-y: auto;
}

.onboarding-container {
  max-inline-size: var(--size-content-3);
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--size-8);
}

.onboarding-step {
  min-block-size: 0;
}

.step-content {
  display: flex;
  flex-direction: column;
  gap: var(--size-5);
}

.step-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
  margin: 0;
}

.step-text {
  font-size: var(--font-size-2);
  color: var(--text-2);
  line-height: 1.6;
  margin: 0;
}

.step-disclaimer {
  font-size: var(--font-size-1);
  color: var(--text-2);
  padding: var(--size-3) var(--size-4);
  background: var(--surface-2);
  border-radius: var(--radius-2);
  border-inline-start: var(--border-size-2) solid var(--yellow-6);
  margin: 0;
}

.onboarding-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.step-indicators {
  display: flex;
  gap: var(--size-2);
}

.step-dot {
  inline-size: var(--size-2);
  block-size: var(--size-2);
  border-radius: 50%;
  background: var(--surface-4);
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
}

.step-dot--active {
  background: var(--indigo-5);
}

.footer-actions {
  display: flex;
  gap: var(--size-2);
}

.btn {
  padding: var(--size-2) var(--size-5);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
  font-family: inherit;
}

.btn--primary {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.btn--ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--ghost:hover {
  color: var(--text-1);
  background: var(--surface-3);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s var(--ease-2);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
