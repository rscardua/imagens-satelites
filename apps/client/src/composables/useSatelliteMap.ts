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

  /** Sobrepõe o raster de uma cena. WPM: janela do viewport em 2 m; demais: footprint. */
  function overlayScene(scene: SceneEntity): void {
    if (!map) return
    removeLayerAndSource('scene-image')

    let corners: Corners | null
    let url: string

    if (scene.source === 'inpe-wpm') {
      // Janela = interseção do viewport com o footprint, renderizada em 2 m nativos.
      const win = clampToFootprint(scene, map.getBounds())
      if (!win) return
      corners = bboxCorners(win)
      url = repository.windowUrl(scene.source, scene.id, win)
    } else {
      if (!scene.previewAsset) return
      corners = footprintCorners(scene)
      if (!corners) return
      url = repository.assetUrl(scene.source, scene.id, scene.previewAsset)
    }

    map.addSource('scene-image', { type: 'image', url, coordinates: corners })
    const before = map.getLayer('footprints-line') ? 'footprints-line' : undefined
    map.addLayer(
      { id: 'scene-image', type: 'raster', source: 'scene-image', paint: { 'raster-opacity': 1 } },
      before,
    )
  }

  /** Interseção [minLon,minLat,maxLon,maxLat] do viewport com o footprint da cena. */
  function clampToFootprint(
    scene: SceneEntity,
    bounds: maplibregl.LngLatBounds,
  ): [number, number, number, number] | null {
    const fc = footprintCorners(scene)
    if (!fc) return null
    const fMinLon = fc[3][0]
    const fMinLat = fc[3][1]
    const fMaxLon = fc[1][0]
    const fMaxLat = fc[1][1]
    const minLon = Math.max(bounds.getWest(), fMinLon)
    const minLat = Math.max(bounds.getSouth(), fMinLat)
    const maxLon = Math.min(bounds.getEast(), fMaxLon)
    const maxLat = Math.min(bounds.getNorth(), fMaxLat)
    if (minLon >= maxLon || minLat >= maxLat) return null
    return [minLon, minLat, maxLon, maxLat]
  }

  function bboxCorners(b: [number, number, number, number]): Corners {
    const [minLon, minLat, maxLon, maxLat] = b
    return [
      [minLon, maxLat],
      [maxLon, maxLat],
      [maxLon, minLat],
      [minLon, minLat],
    ]
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
