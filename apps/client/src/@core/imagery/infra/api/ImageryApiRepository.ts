import { z } from 'zod'

import { HttpClient } from '../../../@shared/infra/http/http-client'
import { boundsToBBox } from '../../domain/entities/MapBounds'
import type {
  ImagerySearchResult,
  SceneEntity,
  SourceId,
} from '../../domain/entities/SceneEntity'
import type {
  ImageryRepositoryInterface,
  SearchImageryInput,
} from '../../domain/repository/ImageryRepositoryInterface'

const sourceSchema = z.enum(['inpe', 'inpe-wpm', 'nasa'])

const sceneSchema = z.object({
  id: z.string(),
  acquired_at: z.string(),
  source: sourceSchema,
  sensor: z.string(),
  cloud_cover: z.number().optional(),
  footprint: z.unknown(),
  preview_asset: z.enum(['thumbnail', 'preview']).optional(),
})

const tileLayerSchema = z.object({
  source: sourceSchema,
  url_template: z.string(),
  layer: z.string(),
  date: z.string(),
  max_zoom: z.number(),
  attribution: z.string(),
})

const resultSchema = z.object({
  source: sourceSchema,
  scenes: z.array(sceneSchema),
  prioritized_scene_id: z.string().optional(),
  tile_layer: tileLayerSchema.optional(),
  truncated: z.boolean(),
})

export class ImageryApiRepository implements ImageryRepositoryInterface {
  constructor(private readonly http: HttpClient = new HttpClient()) {}

  async search(input: SearchImageryInput): Promise<ImagerySearchResult> {
    const body = {
      bbox: boundsToBBox(input.bounds),
      source: input.source,
      date_from: input.range?.from,
      date_to: input.range?.to,
      limit: input.limit,
    }
    const raw = await this.http.post<unknown>('/api/imagery/search', body)
    return mapResult(resultSchema.parse(raw))
  }

  assetUrl(source: SourceId, sceneId: string, asset: 'thumbnail' | 'preview'): string {
    const q = new URLSearchParams({ source, scene_id: sceneId, asset })
    return this.http.url(`/api/imagery/assets?${q.toString()}`)
  }

  overviewUrl(source: SourceId, sceneId: string, size = 2048): string {
    const q = new URLSearchParams({ source, scene_id: sceneId, size: String(size) })
    return this.http.url(`/api/imagery/overview?${q.toString()}`)
  }

  windowUrl(
    source: SourceId,
    sceneId: string,
    bbox: [number, number, number, number],
    size = 1024,
  ): string {
    const q = new URLSearchParams({
      source,
      scene_id: sceneId,
      bbox: bbox.join(','),
      size: String(size),
    })
    return this.http.url(`/api/imagery/window?${q.toString()}`)
  }
}

type RawResult = z.infer<typeof resultSchema>
type RawScene = z.infer<typeof sceneSchema>

function mapScene(s: RawScene): SceneEntity {
  return {
    id: s.id,
    acquiredAt: s.acquired_at,
    source: s.source,
    sensor: s.sensor,
    cloudCover: s.cloud_cover,
    footprint: s.footprint as SceneEntity['footprint'],
    previewAsset: s.preview_asset,
  }
}

function mapResult(r: RawResult): ImagerySearchResult {
  return {
    source: r.source,
    scenes: r.scenes.map(mapScene),
    prioritizedSceneId: r.prioritized_scene_id,
    tileLayer: r.tile_layer
      ? {
          source: r.tile_layer.source,
          urlTemplate: r.tile_layer.url_template,
          layer: r.tile_layer.layer,
          date: r.tile_layer.date,
          maxZoom: r.tile_layer.max_zoom,
          attribution: r.tile_layer.attribution,
        }
      : undefined,
    truncated: r.truncated,
  }
}
