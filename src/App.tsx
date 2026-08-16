import { createSignal, onCleanup, onMount, createMemo, Show, lazy, Suspense } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { check } from '@tauri-apps/plugin-updater'
import { Sidebar } from './components/Sidebar'
import type { NavKey } from './components/Sidebar'
import { Dashboard } from './components/Dashboard'
import { SettingsPanel } from './components/SettingsPanel'
import { UpdatesPanel } from './components/UpdatesPanel'
import { AboutPanel } from './components/AboutPanel'
import { t } from './i18n'
import './App.css'

const LicensesPanel = lazy(() => import('./components/LicensesPanel'))

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
  auto_start: boolean
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
  const [activeTab, setActiveTab] = createSignal<NavKey>('dashboard')
  const [navCollapsed, setNavCollapsed] = createSignal(false)

  const saved = typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_RPC_KEY) : null
  const [rpcEnabled, setRpcEnabled] = createSignal(saved === 'true')

  const [traySettings, setTraySettings] = createSignal<AppSettings>({
    auto_start: false,
    start_in_tray: true,
    minimize_to_tray: true,
    close_to_tray: true,
  })

  let pollingTimer: ReturnType<typeof setInterval> | undefined
  let lastPresenceKey: string | null = null

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
        const key = `${result.title}|${result.artist}|${result.album}|${result.is_playing}`
        if (key !== lastPresenceKey) {
          lastPresenceKey = key
          await invoke('update_discord_presence', { info: result })
        }
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

  async function updateSetting(key: keyof AppSettings, value: boolean | string | null) {
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

    check().then(update => {
      if (update && Notification.permission === 'granted') {
        new Notification('WSA RPC Bridge', {
          body: t('updates.update_available_notification', { version: update.version })
        })
      }
    })

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
    <div id="app" class="shell">
      <Sidebar
        active={activeTab()}
        collapsed={navCollapsed()}
        onSelect={setActiveTab}
        onToggleCollapsed={() => setNavCollapsed((c) => !c)}
      />

      <main class="content">
        <Show when={activeTab() === 'dashboard'}>
          <Dashboard
            media={media()}
            loading={loading()}
            error={error()}
            displayPosition={displayPosition()}
            adbConnected={adbConnected()}
            discordConnected={discordConnected()}
            rpcEnabled={rpcEnabled()}
            onRetry={fetchMediaInfo}
          />
        </Show>

        <Show when={activeTab() === 'settings'}>
          <SettingsPanel
            rpcEnabled={rpcEnabled()}
            onRpcChange={handleRpcChange}
            traySettings={traySettings()}
            onUpdateSetting={updateSetting}
          />
        </Show>

        <Show when={activeTab() === 'updates'}>
          <UpdatesPanel />
        </Show>

        <Show when={activeTab() === 'licenses'}>
          <Suspense fallback={<p class="panel-loading">{t('licenses.loading')}</p>}>
            <LicensesPanel />
          </Suspense>
        </Show>

        <Show when={activeTab() === 'about'}>
          <AboutPanel />
        </Show>

        <Show when={error() && media()}>
          <p class="error-toast">{error()}</p>
        </Show>
      </main>
    </div>
  )
}

export default App