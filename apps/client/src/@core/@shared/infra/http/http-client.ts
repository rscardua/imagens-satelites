import axios, { type AxiosInstance } from 'axios'

/** Wrapper fino do Axios. Base configurável; default usa o proxy `/api` do Vite. */
export class HttpClient {
  private readonly client: AxiosInstance

  constructor(baseURL = '') {
    this.client = axios.create({ baseURL, timeout: 15000 })
  }

  async post<T>(url: string, body: unknown): Promise<T> {
    const { data } = await this.client.post<T>(url, body)
    return data
  }

  /** Monta uma URL absoluta/relativa para recursos servidos pelo backend (assets/tiles). */
  url(path: string): string {
    return `${this.client.defaults.baseURL ?? ''}${path}`
  }
}
