import { createSignal, onCleanup, onMount, Show, createMemo } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Tabs } from '@kobalte/core/tabs'
import { Switch } from '@kobalte/core/switch'
import './App.css'

interface MediaInfo {
  title: string
  artist: string
  album: string
  package_name: string
  thumbnail_url: string | null
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

interface AppSettings {
  start_in_tray: boolean
  minimize_to_tray: boolean
  close_to_tray: boolean
}

function App() {
  const [adbConnected, setAdbConnected] = createSignal(false)
  const [discordConnected, setDiscordConnected] = createSignal(false)
  const [media, setMedia] = createSignal<MediaInfo | null>(null)
  const [error, setError] = createSignal<string | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [lastFetch, setLastFetch] = createSignal<{ pos: number; time: number } | null>(null)
  const [now, setNow] = createSignal(Date.now())
  const [activeTab, setActiveTab] = createSignal('media')

  const saved = typeof localStorage !== 'undefined' ? localStorage.getItem('rpcEnabled') : null
  const [rpcEnabled, setRpcEnabled] = createSignal(saved === 'true')

  const [traySettings, setTraySettings] = createSignal<AppSettings>({
    start_in_tray: true,
    minimize_to_tray: true,
    close_to_tray: true,
  })

  let pollingTimer: ReturnType<typeof setInterval> | undefined

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
    try {
      const result = await invoke<MediaInfo>('get_media_info')
      setMedia(result)
      setError(null)
      setAdbConnected(true)
      if (result.position !== null) {
        setLastFetch({ pos: result.position, time: Date.now() })
      }
      if (rpcEnabled()) {
        await invoke('connect_discord')
        await invoke('update_discord_presence', { info: result })
      }
    } catch (e) {
      setMedia(null)
      setLastFetch(null)
      setError(String(e))
      if (rpcEnabled()) {
        await invoke('disconnect_discord')
      }
    } finally {
      setLoading(false)
    }
  }

  async function handleRpcChange(enabled: boolean) {
    setRpcEnabled(enabled)
    localStorage.setItem('rpcEnabled', String(enabled))
    try {
      if (enabled) {
        await invoke('connect_discord')
        const current = media()
        if (current) {
          await invoke('update_discord_presence', { info: current })
        }
      } else {
        await invoke('disconnect_discord')
      }
    } catch (e) {
      console.error('rpc toggle failed', e)
    }
  }

  async function loadSettings() {
    try {
      const s = await invoke<AppSettings>('get_settings')
      setTraySettings(s)
    } catch (e) {
      console.error('failed to load settings', e)
    }
  }

  async function updateSetting(key: keyof AppSettings, value: boolean) {
    const next = { ...traySettings(), [key]: value }
    setTraySettings(next)
    try {
      await invoke('update_settings', { config: next })
    } catch (e) {
      console.error('failed to save settings', e)
    }
  }

  onMount(async () => {
    await checkStatus()

    const unlisten = await listen('show-settings', () => {
      setActiveTab('settings')
    })
    onCleanup(unlisten)

    await loadSettings()

    if (rpcEnabled()) {
      try {
        await invoke('connect_discord')
      } catch (e) {
        console.error('initial connect_discord failed', e)
      }
    }
    await fetchMediaInfo()
    pollingTimer = setInterval(async () => {
      await checkStatus()
      await fetchMediaInfo()
    }, 5000)
    onCleanup(() => {
      if (pollingTimer) clearInterval(pollingTimer)
    })
  })

  return (
    <div id="app">
      <header>
        <h1>WSA RPC Bridge</h1>
      </header>

      <div id="status-bar">
        <span class={`dot ${adbConnected() ? 'connected' : 'disconnected'}`} />
        <span>ADB</span>
        <span class="status-value">{adbConnected() ? '接続済み' : '切断'}</span>

        <span class="status-sep">|</span>

        <span class={`dot ${discordConnected() ? 'connected' : 'disconnected'}`} />
        <span>Discord RPC</span>
        <span class="status-value">
          {discordConnected() ? '接続済み' : rpcEnabled() ? '待機中' : '切断'}
        </span>
      </div>

      <Tabs value={activeTab()} onChange={setActiveTab} class="tabs">
        <Tabs.List class="tabs-list" aria-label="tabs">
          <Tabs.Trigger class="tab-trigger" value="media">再生中</Tabs.Trigger>
          <Tabs.Trigger class="tab-trigger" value="settings">設定</Tabs.Trigger>
          <Tabs.Indicator class="tab-indicator" />
        </Tabs.List>

        <Tabs.Content class="tab-content" value="media">
          <Show when={loading() && !media()} fallback={
            <Show when={media()} fallback={
              <div class="empty-state">
                <p>{error() ?? '再生中のメディアはありません'}</p>
                <button onClick={fetchMediaInfo} class="btn">再試行</button>
              </div>
            }>
              {(m) => (
                <div class="media-card">
                  <Show when={m().thumbnail_url}>
                    <img src={m().thumbnail_url!} alt="album art" class="thumb" />
                  </Show>
                  <div class="media-body">
                    <div class="track-title">
                      <span class={`play-icon ${m().is_playing ? 'playing' : 'paused'}`}>
                        {m().is_playing ? '▶' : '⏸'}
                      </span>
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
                </div>
              )}
            </Show>
          }>
            <div class="loading-msg">読み込み中...</div>
          </Show>

          <Show when={error() && media()}>
            <p class="error-toast">{error()}</p>
          </Show>
        </Tabs.Content>

        <Tabs.Content class="tab-content" value="settings">
          <div class="settings-card">
            <Switch
              checked={rpcEnabled()}
              onChange={handleRpcChange}
              class="rpc-switch"
            >
              <Switch.Label class="switch-label">
                Discord Rich Presence を有効にする
              </Switch.Label>
              <Switch.Control class="switch-track">
                <Switch.Thumb class="switch-thumb" />
              </Switch.Control>
            </Switch>
            <p class="switch-desc">
              再生中のメディア情報を Discord のアクティビティに表示します。
            </p>
          </div>

          <div class="settings-divider" />

          <h3 class="section-title">タスクトレイ設定</h3>

          <div class="settings-card">
            <Switch
              checked={traySettings().start_in_tray}
              onChange={(v) => updateSetting('start_in_tray', v)}
              class="rpc-switch"
            >
              <Switch.Label class="switch-label">
                起動時にタスクトレイに格納
              </Switch.Label>
              <Switch.Control class="switch-track">
                <Switch.Thumb class="switch-thumb" />
              </Switch.Control>
            </Switch>

            <Switch
              checked={traySettings().minimize_to_tray}
              onChange={(v) => updateSetting('minimize_to_tray', v)}
              class="rpc-switch"
            >
              <Switch.Label class="switch-label">
                最小化時にタスクトレイに収納
              </Switch.Label>
              <Switch.Control class="switch-track">
                <Switch.Thumb class="switch-thumb" />
              </Switch.Control>
            </Switch>

            <Switch
              checked={traySettings().close_to_tray}
              onChange={(v) => updateSetting('close_to_tray', v)}
              class="rpc-switch"
            >
              <Switch.Label class="switch-label">
                閉じる時にタスクトレイに収納
              </Switch.Label>
              <Switch.Control class="switch-track">
                <Switch.Thumb class="switch-thumb" />
              </Switch.Control>
            </Switch>
          </div>
        </Tabs.Content>
      </Tabs>
    </div>
  )
}

export default App
