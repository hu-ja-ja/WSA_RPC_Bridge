import { createSignal, onCleanup, onMount, Show, createMemo } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import './App.css'

interface MediaInfo {
  title: string
  artist: string
  album: string
  position: number | null
  duration: number | null
  is_playing: boolean
}

function formatTime(ms: number): string {
  const totalSec = Math.floor(ms / 1000)
  const min = Math.floor(totalSec / 60)
  const sec = totalSec % 60
  return `${min}:${sec.toString().padStart(2, '0')}`
}

function App() {
  const [adbConnected, setAdbConnected] = createSignal(false)
  const [discordConnected, setDiscordConnected] = createSignal(false)
  const [media, setMedia] = createSignal<MediaInfo | null>(null)
  const [error, setError] = createSignal<string | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [lastFetch, setLastFetch] = createSignal<{ pos: number; time: number } | null>(null)
  const [now, setNow] = createSignal(Date.now())

  onMount(() => {
    const tick = setInterval(() => setNow(Date.now()), 1000)
    onCleanup(() => clearInterval(tick))
  })

  const displayPosition = createMemo(() => {
    const m = media()
    if (!m || m.position === null) return null
    const lf = lastFetch()
    if (!lf) return m.position
    if (!m.is_playing) return lf.pos
    const elapsed = now() - lf.time
    return lf.pos + elapsed
  })

  async function checkStatus() {
    try {
      const [adb, dc] = await Promise.all([
        invoke<boolean>('get_adb_status'),
        invoke<boolean>('get_discord_status'),
      ])
      setAdbConnected(adb)
      setDiscordConnected(dc)
    } catch (e) {
      console.error('status check failed', e)
    }
  }

  async function fetchMediaInfo() {
    setLoading(true)
    setError(null)
    try {
      const result = await invoke<MediaInfo>('get_media_info')
      setMedia(result)
      setAdbConnected(true)
      if (result.position !== null) {
        setLastFetch({ pos: result.position, time: Date.now() })
      }
      await invoke('update_discord_presence', { info: result })
    } catch (e) {
      setMedia(null)
      setAdbConnected(false)
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  onMount(async () => {
    await checkStatus()
    await fetchMediaInfo()
    const interval = setInterval(async () => {
      await checkStatus()
      await fetchMediaInfo()
    }, 5000)
    onCleanup(() => clearInterval(interval))
  })

  return (
    <div id="app">
      <header>
        <h1>WSA RPC Bridge</h1>
      </header>

      <section id="status">
        <div class="status-row">
          <span class="label">ADB</span>
          <span class={`dot ${adbConnected() ? 'connected' : 'disconnected'}`} />
          <span class="value">{adbConnected() ? 'Connected' : 'Disconnected'}</span>
        </div>
        <div class="status-row">
          <span class="label">Discord</span>
          <span class={`dot ${discordConnected() ? 'connected' : 'disconnected'}`} />
          <span class="value">{discordConnected() ? 'Connected' : 'Disconnected'}</span>
        </div>
      </section>

      <section id="media">
        <Show when={loading() && !media()} fallback={
          <Show when={media()} fallback={
            <div class="empty">
              <p>{error() ?? 'No media information available'}</p>
              <button onClick={fetchMediaInfo} class="retry-btn">Retry</button>
            </div>
          }>
            {(m) => (
              <div class="media-info">
                <div class="track-title">
                  <span class={`play-indicator ${m().is_playing ? 'playing' : 'paused'}`} />
                  {m().title}
                </div>
                <div class="artist">{m().artist}</div>
                <div class="album">{m().album}</div>
                <div class="position">
                  <Show when={displayPosition() !== null}>
                    <span>{formatTime(displayPosition()!)}</span>
                  </Show>
                  <Show when={m().duration !== null}>
                    <span> / {formatTime(m().duration!)}</span>
                  </Show>
                </div>
              </div>
            )}
          </Show>
        }>
          <div class="loading">Fetching media info...</div>
        </Show>
      </section>

      <Show when={error() && media()}>
        <p class="error-msg">{error()}</p>
      </Show>
    </div>
  )
}

export default App
