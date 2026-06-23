<script setup lang="ts">
import type { SceneEntity } from '@/@core/imagery/domain/entities/SceneEntity'

defineProps<{ scene: SceneEntity | null }>()

const sourceLabel: Record<string, string> = { inpe: 'INPE (CBERS-4A)', nasa: 'NASA (GIBS)' }
</script>

<template>
  <aside
    v-if="scene"
    class="scene-info"
  >
    <h3>Cena selecionada</h3>
    <dl>
      <dt>Data de aquisição</dt>
      <dd>{{ new Date(scene.acquiredAt).toLocaleString() }}</dd>
      <dt>Sensor</dt>
      <dd>{{ scene.sensor }}</dd>
      <dt>Fonte</dt>
      <dd>{{ sourceLabel[scene.source] ?? scene.source }}</dd>
      <template v-if="scene.cloudCover !== undefined">
        <dt>Cobertura de nuvens</dt>
        <dd>{{ scene.cloudCover.toFixed(1) }}%</dd>
      </template>
    </dl>
  </aside>
</template>

<style scoped>
.scene-info {
  position: absolute;
  top: 1rem;
  right: 1rem;
  z-index: 1000;
  background: #fff;
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  max-width: 260px;
  font-size: 0.875rem;
}
dt {
  font-weight: 600;
  margin-top: 0.4rem;
}
</style>
