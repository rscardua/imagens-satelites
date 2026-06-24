import type { DateRange } from '../entities/DateRange'
import type { MapBounds } from '../entities/MapBounds'
import type { ImagerySearchResult, SourceId } from '../entities/SceneEntity'

export interface SearchImageryInput {
  bounds: MapBounds
  source: SourceId
  range?: DateRange
  limit?: number
}

/** Contrato que a camada de infraestrutura (API) deve implementar. */
export interface ImageryRepositoryInterface {
  search(input: SearchImageryInput): Promise<ImagerySearchResult>
  /** URL para o asset visual de uma cena (proxy do backend). */
  assetUrl(source: SourceId, sceneId: string, asset: 'thumbnail' | 'preview'): string
  /** URL para o overview PNG renderizado do COG (maior resolução). */
  overviewUrl(source: SourceId, sceneId: string, size?: number): string
  /** URL para render de janela geográfica em resolução nativa (2 m). */
  windowUrl(
    source: SourceId,
    sceneId: string,
    bbox: [number, number, number, number],
    size?: number,
  ): string
}
