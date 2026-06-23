<script setup lang="ts">
import { computed } from 'vue'

import type { DateRange } from '@/@core/imagery/domain/entities/DateRange'

const props = defineProps<{ modelValue: DateRange }>()
const emit = defineEmits<{ 'update:modelValue': [DateRange] }>()

// Inputs HTML usam YYYY-MM-DD; o modelo guarda ISO completo.
const fromDate = computed({
  get: () => props.modelValue.from.slice(0, 10),
  set: (v: string) => emit('update:modelValue', { ...props.modelValue, from: `${v}T00:00:00Z` }),
})
const toDate = computed({
  get: () => props.modelValue.to.slice(0, 10),
  set: (v: string) => emit('update:modelValue', { ...props.modelValue, to: `${v}T23:59:59Z` }),
})
</script>

<template>
  <div class="date-filter">
    <label>De <input
      v-model="fromDate"
      type="date"
    ></label>
    <label>Até <input
      v-model="toDate"
      type="date"
    ></label>
  </div>
</template>

<style scoped>
.date-filter {
  display: flex;
  gap: 0.5rem;
  font-size: 0.78rem;
}
input {
  margin-left: 0.25rem;
}
</style>
