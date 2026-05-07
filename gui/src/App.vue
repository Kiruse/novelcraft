<template>
  <Onboarding v-if="!completed" />
  <Main v-else />
</template>

<script setup lang="ts">
import Main from '~/Main.vue';
import Onboarding from '~/Onboarding.vue';
import { useOnboarding } from '~/composables/useOnboarding';

const { completed } = useOnboarding();
</script>
