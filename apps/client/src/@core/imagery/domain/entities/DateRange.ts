/** Intervalo de datas para filtrar cenas (ISO 8601). */
export interface DateRange {
  from: string
  to: string
}

/** Janela padrão: últimos N dias até agora (default 30 — assumption da spec). */
export function defaultDateRange(days = 30): DateRange {
  const to = new Date()
  const from = new Date(to.getTime() - days * 24 * 60 * 60 * 1000)
  return { from: from.toISOString(), to: to.toISOString() }
}
