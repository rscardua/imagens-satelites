import { describe, expect, it } from 'vitest'

import { SearchImageryUseCase } from '../application/use-case/SearchImageryUseCase'
import type { ImagerySearchResult, SourceId } from '../domain/entities/SceneEntity'
import type {
  ImageryRepositoryInterface,
  SearchImageryInput,
} from '../domain/repository/ImageryRepositoryInterface'

class FakeRepository implements ImageryRepositoryInterface {
  public lastInput?: SearchImageryInput

  constructor(private readonly result: ImagerySearchResult) {}

  async search(input: SearchImageryInput): Promise<ImagerySearchResult> {
    this.lastInput = input
    return this.result
  }

  assetUrl(source: SourceId, sceneId: string, asset: 'thumbnail' | 'preview'): string {
    return `/api/imagery/assets?source=${source}&scene_id=${sceneId}&asset=${asset}`
  }
}

describe('SearchImageryUseCase', () => {
  const input: SearchImageryInput = {
    bounds: { minLon: -48, minLat: -16, maxLon: -47.9, maxLat: -15.9 },
    source: 'inpe',
  }

  it('delega a busca ao repositório e retorna o resultado', async () => {
    const result: ImagerySearchResult = {
      source: 'inpe',
      scenes: [
        {
          id: 'scene-recent',
          acquiredAt: '2026-05-17T00:00:00Z',
          source: 'inpe',
          sensor: 'WFI',
          footprint: { type: 'Polygon', coordinates: [] } as never,
          previewAsset: 'thumbnail',
        },
      ],
      prioritizedSceneId: 'scene-recent',
      truncated: false,
    }
    const repo = new FakeRepository(result)
    const useCase = new SearchImageryUseCase(repo)

    const out = await useCase.execute(input)

    expect(out.prioritizedSceneId).toBe('scene-recent')
    expect(out.scenes).toHaveLength(1)
    expect(repo.lastInput?.source).toBe('inpe')
  })

  it('propaga resultado vazio sem erro (sem cobertura)', async () => {
    const repo = new FakeRepository({ source: 'inpe', scenes: [], truncated: false })
    const useCase = new SearchImageryUseCase(repo)

    const out = await useCase.execute(input)

    expect(out.scenes).toHaveLength(0)
    expect(out.prioritizedSceneId).toBeUndefined()
  })
})
