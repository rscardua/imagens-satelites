<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { SearchImageryUseCase } from '@/@core/imagery/application/use-case/SearchImageryUseCase'
import { ImageryApiRepository } from '@/@core/imagery/infra/api/ImageryApiRepository'
import { defaultDateRange, type DateRange } from '@/@core/imagery/domain/entities/DateRange'
import type { SceneEntity, SourceId } from '@/@core/imagery/domain/entities/SceneEntity'
import { useSatelliteMap } from '@/composables/useSatelliteMap'
import SceneInfoPanel from '@/components/imagery/SceneInfoPanel.vue'
import SourceSelector from '@/components/imagery/SourceSelector.vue'
import DateRangeFilter from '@/components/imagery/DateRangeFilter.vue'

const repository = new ImageryApiRepository()
const useCase = new SearchImageryUseCase(repository)
const mapEl = ref<HTMLElement | null>(null)
const map = useSatelliteMap(repository)

const source = ref<SourceId>('inpe')
const range = ref<DateRange>(defaultDateRange())
const scenes = ref<SceneEntity[]>([])
const sceneIndex = ref(0)
const selectedScene = ref<SceneEntity | null>(null)
const loading = ref(false)
const message = ref<string | null>(null)
const suggestedSource = ref<SourceId | null>(null)

let debounce: ReturnType<typeof setTimeout> | null = null

async function search(): Promise<void> {
  const bounds = map.currentBounds()
  if (!bounds) return
  loading.value = true
  message.value = null
  suggestedSource.value = null
  try {
    const result = await useCase.execute({ bounds, source: source.value, range: range.value })
    map.render(result)
    scenes.value = result.scenes
    sceneIndex.value = 0
    selectedScene.value = result.scenes[0] ?? null
    if (result.scenes.length === 0 && !result.tileLayer) {
      message.value = 'Nenhuma imagem recente para esta área e período.'
    } else if (result.truncated) {
      message.value = 'Muitos resultados — aproxime o mapa para refinar.'
    }
  } catch (err: unknown) {
    const body = (err as { response?: { data?: { message?: string; suggested_source?: SourceId } } })
      .response?.data
    message.value = body?.message ?? 'Falha ao consultar a fonte de imagens.'
    suggestedSource.value = body?.suggested_source ?? null
  } finally {
    loading.value = false
  }
}

function scheduleSearch(): void {
  if (debounce) clearTimeout(debounce)
  debounce = setTimeout(() => void search(), 400)
}

/** Navega entre cenas sobrepostas por data (US2/AC3). */
function cycleScene(delta: number): void {
  if (scenes.value.length === 0) return
  sceneIndex.value = (sceneIndex.value + delta + scenes.value.length) % scenes.value.length
  const scene = scenes.value[sceneIndex.value]
  if (scene) {
    selectedScene.value = scene
    map.overlayScene(scene)
  }
}

function useSuggested(): void {
  if (suggestedSource.value) {
    source.value = suggestedSource.value
    void search()
  }
}

watch([source, range], () => void search(), { deep: true })

onMounted(() => {
  if (!mapEl.value) return
  map.init(mapEl.value, (scene) => {
    selectedScene.value = scene
  })
  map.onMoveEnd(scheduleSearch)
  void search()
})

onBeforeUnmount(() => {
  if (debounce) clearTimeout(debounce)
  map.destroy()
})
</script>

<template>
  <div class="map-page">
    <div
      ref="mapEl"
      class="map"
    />

    <div class="toolbar">
      <SourceSelector v-model="source" />
      <DateRangeFilter
        v-if="source === 'inpe'"
        v-model="range"
      />
      <div
        v-if="source === 'inpe' && scenes.length > 1"
        class="cycler"
      >
        <button @click="cycleScene(-1)">
          ‹
        </button>
        <span>{{ sceneIndex + 1 }} / {{ scenes.length }}</span>
        <button @click="cycleScene(1)">
          ›
        </button>
      </div>
    </div>

    <SceneInfoPanel :scene="selectedScene" />

    <div
      v-if="loading"
      class="status"
    >
      Carregando imagens…
    </div>
    <div
      v-else-if="message"
      class="status"
    >
      {{ message }}
      <button
        v-if="suggestedSource"
        @click="useSuggested"
      >
        Tentar {{ suggestedSource.toUpperCase() }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.map-page {
  position: relative;
  height: 100vh;
  width: 100vw;
}
.map {
  height: 100%;
  width: 100%;
}
.toolbar {
  position: absolute;
  top: 1rem;
  left: 1rem;
  z-index: 1000;
  display: flex;
  gap: 0.75rem;
  align-items: center;
  background: #fff;
  padding: 0.5rem 0.75rem;
  border-radius: 0.5rem;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}
.cycler {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.8rem;
}
.cycler button {
  width: 1.6rem;
  height: 1.6rem;
  border: 1px solid #cbd5e1;
  background: #fff;
  border-radius: 0.3rem;
  cursor: pointer;
}
.status {
  position: absolute;
  bottom: 1rem;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1000;
  background: #111827;
  color: #fff;
  padding: 0.5rem 1rem;
  border-radius: 0.5rem;
  font-size: 0.875rem;
}
.status button {
  margin-left: 0.5rem;
}
</style>
