import { createSignal, onCleanup, onMount, createMemo, Show } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Tabs } from '@kobalte/core/tabs'
import { StatusBar } from './components/StatusBar'
import { MediaCard } from './components/MediaCard'
import { SettingsPanel } from './components/SettingsPanel'
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

interface AppSettings {
  start_in_tray: boolean
  minimize_to_tray: boolean
  close_to_tray: boolean
}

const POLL_INTERVAL = 5000
const TICK_INTERVAL = 1000
const STORAGE_RPC_KEY = 'rpcEnabled'
const EVENT_SHOW_SETTINGS = 'show-settings'

function App() {
  const [adbConnected, setAdbConnected] = createSignal(false)
  const [discordConnected, setDiscordConnected] = createSignal(false)
  const [media, setMedia] = createSignal<MediaInfo | null>(null)
  const [error, setError] = createSignal<string | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [lastFetch, setLastFetch] = createSignal<{ pos: number; time: number } | null>(null)
  const [now, setNow] = createSignal(Date.now())
  const [activeTab, setActiveTab] = createSignal('media')

  const saved = typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_RPC_KEY) : null
  const [rpcEnabled, setRpcEnabled] = createSignal(saved === 'true')

  const [traySettings, setTraySettings] = createSignal<AppSettings>({
    start_in_tray: true,
    minimize_to_tray: true,
    close_to_tray: true,
  })

  let pollingTimer: ReturnType<typeof setInterval> | undefined

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
    localStorage.setItem(STORAGE_RPC_KEY, String(enabled))
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
    const tick = setInterval(() => setNow(Date.now()), TICK_INTERVAL)
    onCleanup(() => clearInterval(tick))

    const unlisten = await listen(EVENT_SHOW_SETTINGS, () => {
      setActiveTab('settings')
    })
    onCleanup(unlisten)

    await loadSettings()
    await fetchMediaInfo()
    await checkStatus()

    if (rpcEnabled()) {
      try {
        await invoke('connect_discord')
      } catch (e) {
        console.error('initial connect_discord failed', e)
      }
    }

    pollingTimer = setInterval(async () => {
      await checkStatus()
      await fetchMediaInfo()
    }, POLL_INTERVAL)
    onCleanup(() => {
      if (pollingTimer) clearInterval(pollingTimer)
    })
  })

  return (
    <div id="app">
      <header>
        <h1>WSA RPC Bridge</h1>
      </header>

      <StatusBar
        adbConnected={adbConnected()}
        discordConnected={discordConnected()}
        rpcEnabled={rpcEnabled()}
      />

      <Tabs value={activeTab()} onChange={setActiveTab} class="tabs">
        <Tabs.List class="tabs-list" aria-label="tabs">
          <Tabs.Trigger class="tab-trigger" value="media">再生中</Tabs.Trigger>
          <Tabs.Trigger class="tab-trigger" value="settings">設定</Tabs.Trigger>
          <Tabs.Indicator class="tab-indicator" />
        </Tabs.List>

        <Tabs.Content class="tab-content" value="media">
          <MediaCard
            media={media()}
            loading={loading()}
            error={error()}
            displayPosition={displayPosition()}
            onRetry={fetchMediaInfo}
          />

          <Show when={error() && media()}>
            <p class="error-toast">{error()}</p>
          </Show>
        </Tabs.Content>

        <Tabs.Content class="tab-content" value="settings">
          <SettingsPanel
            rpcEnabled={rpcEnabled()}
            onRpcChange={handleRpcChange}
            traySettings={traySettings()}
            onUpdateSetting={updateSetting}
          />
        </Tabs.Content>
      </Tabs>
    </div>
  )
}

export default App
