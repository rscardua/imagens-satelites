/** Extensão geográfica visível do mapa (EPSG:4326). */
export interface MapBounds {
  minLon: number
  minLat: number
  maxLon: number
  maxLat: number
}

/** Serializa a bbox no formato esperado pela API: [minLon, minLat, maxLon, maxLat]. */
export function boundsToBBox(b: MapBounds): [number, number, number, number] {
  return [b.minLon, b.minLat, b.maxLon, b.maxLat]
}
