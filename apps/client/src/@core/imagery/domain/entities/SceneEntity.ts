export type SourceId = 'inpe' | 'nasa'

/** Cena de satélite exibida no mapa. */
export interface SceneEntity {
  id: string
  acquiredAt: string
  source: SourceId
  sensor: string
  cloudCover?: number
  footprint: GeoJSON.GeoJsonObject
  previewAsset?: 'thumbnail' | 'preview'
}

/** Descritor de camada de tiles (NASA GIBS). */
export interface TileLayerDescriptor {
  source: SourceId
  urlTemplate: string
  layer: string
  date: string
  maxZoom: number
  attribution: string
}

/** Resultado de uma busca de imagens. */
export interface ImagerySearchResult {
  source: SourceId
  scenes: SceneEntity[]
  prioritizedSceneId?: string
  tileLayer?: TileLayerDescriptor
  truncated: boolean
}
