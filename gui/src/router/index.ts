import { createRouter, createWebHistory } from 'vue-router';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('~/pages/index.vue'),
    },
    {
      path: '/vignettes',
      name: 'vignettes',
      component: () => import('~/pages/vignettes/index.vue'),
    },
    {
      path: '/vignettes/:id',
      name: 'vignette',
      component: () => import('~/pages/vignettes/[id].vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('~/pages/settings.vue'),
    },
    {
      path: '/builder',
      name: 'builder',
      component: () => import('~/pages/builder.vue'),
    },
  ],
});

export { router };
