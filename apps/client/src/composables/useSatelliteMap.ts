import maplibregl from 'maplibre-gl'

import type {
  ImagerySearchResult,
  SceneEntity,
} from '@/@core/imagery/domain/entities/SceneEntity'
import type { MapBounds } from '@/@core/imagery/domain/entities/MapBounds'
import type { ImageryRepositoryInterface } from '@/@core/imagery/domain/repository/ImageryRepositoryInterface'

type Corners = [[number, number], [number, number], [number, number], [number, number]]

const OSM_STYLE: maplibregl.StyleSpecification = {
  version: 8,
  sources: {
    osm: {
      type: 'raster',
      tiles: [
        'https://a.tile.openstreetmap.org/{z}/{x}/{y}.png',
        'https://b.tile.openstreetmap.org/{z}/{x}/{y}.png',
        'https://c.tile.openstreetmap.org/{z}/{x}/{y}.png',
      ],
      tileSize: 256,
      attribution: '© OpenStreetMap contributors',
    },
  },
  layers: [{ id: 'osm', type: 'raster', source: 'osm' }],
}

/**
 * Encapsula o MapLibre GL: camada base (OSM), footprints (GeoJSON line),
 * overlay raster da cena priorizada (image source) e camada de tiles (NASA GIBS).
 */
export function useSatelliteMap(repository: ImageryRepositoryInterface) {
  let map: maplibregl.Map | null = null
  let ready = false
  let pending: ImagerySearchResult | null = null
  let sceneById = new Map<string, SceneEntity>()
  let onSceneClick: ((scene: SceneEntity) => void) | null = null

  function init(el: HTMLElement, sceneClick: (scene: SceneEntity) => void): void {
    onSceneClick = sceneClick
    map = new maplibregl.Map({
      container: el,
      style: OSM_STYLE,
      center: [-47.92, -15.78],
      zoom: 7,
    })
    map.addControl(new maplibregl.NavigationControl(), 'top-right')
    map.on('load', () => {
      ready = true
      if (pending) {
        applyResult(pending)
        pending = null
      }
    })
    map.on('click', 'footprints-line', (e) => {
      const id = e.features?.[0]?.properties?.sceneId as string | undefined
      const scene = id ? sceneById.get(id) : undefined
      if (scene && onSceneClick) onSceneClick(scene)
    })
    map.on('mouseenter', 'footprints-line', () => setCursor('pointer'))
    map.on('mouseleave', 'footprints-line', () => setCursor(''))
  }

  function setCursor(value: string): void {
    if (map) map.getCanvas().style.cursor = value
  }

  function currentBounds(): MapBounds | null {
    if (!map) return null
    const b = map.getBounds()
    return { minLon: b.getWest(), minLat: b.getSouth(), maxLon: b.getEast(), maxLat: b.getNorth() }
  }

  function onMoveEnd(handler: () => void): void {
    map?.on('moveend', handler)
  }

  function render(result: ImagerySearchResult): void {
    if (!ready) {
      pending = result
      return
    }
    applyResult(result)
  }

  function applyResult(result: ImagerySearchResult): void {
    if (!map) return
    sceneById = new Map(result.scenes.map((s) => [s.id, s]))
    removeLayerAndSource('scene-image')
    removeLayerAndSource('gibs-layer', 'gibs')

    if (result.tileLayer) {
      removeLayerAndSource('footprints-line', 'footprints')
      map.addSource('gibs', {
        type: 'raster',
        tiles: [result.tileLayer.urlTemplate],
        tileSize: 256,
        attribution: result.tileLayer.attribution,
      })
      map.addLayer({ id: 'gibs-layer', type: 'raster', source: 'gibs' })
      return
    }

    const fc: GeoJSON.FeatureCollection = {
      type: 'FeatureCollection',
      features: result.scenes.map((s) => ({
        type: 'Feature',
        geometry: s.footprint as GeoJSON.Geometry,
        properties: { sceneId: s.id },
      })),
    }
    const source = map.getSource('footprints') as maplibregl.GeoJSONSource | undefined
    if (source) {
      source.setData(fc)
    } else {
      map.addSource('footprints', { type: 'geojson', data: fc })
      map.addLayer({
        id: 'footprints-line',
        type: 'line',
        source: 'footprints',
        paint: { 'line-color': '#2563eb', 'line-width': 1 },
      })
    }

    const prioritized = result.scenes.find((s) => s.id === result.prioritizedSceneId)
    if (prioritized) overlayScene(prioritized)
  }

  /** Sobrepõe o raster (preview) de uma cena específica (navegação por data — US2). */
  function overlayScene(scene: SceneEntity): void {
    if (!map) return
    removeLayerAndSource('scene-image')
    if (!scene.previewAsset) return
    const corners = footprintCorners(scene)
    if (!corners) return
    map.addSource('scene-image', {
      type: 'image',
      url: repository.assetUrl(scene.source, scene.id, scene.previewAsset),
      coordinates: corners,
    })
    // Abaixo do contorno (footprints-line) quando existir.
    const before = map.getLayer('footprints-line') ? 'footprints-line' : undefined
    map.addLayer(
      { id: 'scene-image', type: 'raster', source: 'scene-image', paint: { 'raster-opacity': 0.9 } },
      before,
    )
  }

  function footprintCorners(scene: SceneEntity): Corners | null {
    const coords: number[][] = []
    collectCoords(scene.footprint as GeoJSON.Geometry, coords)
    if (coords.length === 0) return null
    const lons = coords.map((c) => c[0] ?? 0)
    const lats = coords.map((c) => c[1] ?? 0)
    const minLon = Math.min(...lons)
    const maxLon = Math.max(...lons)
    const minLat = Math.min(...lats)
    const maxLat = Math.max(...lats)
    // Ordem MapLibre: topo-esq, topo-dir, baixo-dir, baixo-esq.
    return [
      [minLon, maxLat],
      [maxLon, maxLat],
      [maxLon, minLat],
      [minLon, minLat],
    ]
  }

  function removeLayerAndSource(layerId: string, sourceId?: string): void {
    if (!map) return
    if (map.getLayer(layerId)) map.removeLayer(layerId)
    const sid = sourceId ?? layerId
    if (map.getSource(sid)) map.removeSource(sid)
  }

  function destroy(): void {
    map?.remove()
    map = null
    ready = false
  }

  return { init, render, overlayScene, currentBounds, onMoveEnd, destroy }
}

function collectCoords(geom: GeoJSON.Geometry, out: number[][]): void {
  if (geom.type === 'Polygon') {
    for (const ring of geom.coordinates) for (const p of ring) out.push(p)
  } else if (geom.type === 'MultiPolygon') {
    for (const poly of geom.coordinates) for (const ring of poly) for (const p of ring) out.push(p)
  }
}
