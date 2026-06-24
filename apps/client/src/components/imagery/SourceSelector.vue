<script setup lang="ts">
import type { SourceId } from '@/@core/imagery/domain/entities/SceneEntity'

defineProps<{ modelValue: SourceId }>()
const emit = defineEmits<{ 'update:modelValue': [SourceId] }>()

const options: { id: SourceId; label: string }[] = [
  { id: 'inpe', label: 'INPE · WFI 64m' },
  { id: 'inpe-wpm', label: 'INPE · WPM 2m' },
  { id: 'nasa', label: 'NASA · GIBS' },
]
</script>

<template>
  <div class="source-selector">
    <button
      v-for="opt in options"
      :key="opt.id"
      :class="{ active: modelValue === opt.id }"
      @click="emit('update:modelValue', opt.id)"
    >
      {{ opt.label }}
    </button>
  </div>
</template>

<style scoped>
.source-selector {
  display: flex;
  gap: 0.25rem;
}
button {
  padding: 0.35rem 0.7rem;
  border: 1px solid #cbd5e1;
  background: #fff;
  border-radius: 0.4rem;
  cursor: pointer;
  font-size: 0.8rem;
}
button.active {
  background: #2563eb;
  color: #fff;
  border-color: #2563eb;
}
</style>
