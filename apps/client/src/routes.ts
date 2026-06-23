import type { RouteRecordRaw } from 'vue-router'

/** Catálogo declarativo de rotas (acesso público na v1). */
export const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'mapa',
    component: () => import('@/pages/Mapa/Index.vue'),
    meta: { isPublic: true, title: 'Mapa de Imagens CBERS-4A' },
  },
]
