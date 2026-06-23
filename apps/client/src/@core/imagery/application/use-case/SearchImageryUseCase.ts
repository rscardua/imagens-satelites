import type {
  ImageryRepositoryInterface,
  SearchImageryInput,
} from '../../domain/repository/ImageryRepositoryInterface'
import type { ImagerySearchResult } from '../../domain/entities/SceneEntity'

/** Caso de uso de UX: busca cenas/camada para a área visível. */
export class SearchImageryUseCase {
  constructor(private readonly repository: ImageryRepositoryInterface) {}

  execute(input: SearchImageryInput): Promise<ImagerySearchResult> {
    return this.repository.search(input)
  }
}
