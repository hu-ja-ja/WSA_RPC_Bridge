import { createSignal, onCleanup, onMount, createMemo, Show, lazy, Suspense } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { check } from '@tauri-apps/plugin-updater'
import { Sidebar } from './components/Sidebar'
import type { NavKey } from './components/Sidebar'
import { Menu } from 'lucide-solid'
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
  media_whitelist: string[]
  media_notification: boolean
}

const POLL_INTERVAL = 5000
const TICK_INTERVAL = 1000
const STORAGE_RPC_KEY = 'rpcEnabled'
const EVENT_SHOW_SETTINGS = 'show-settings'
const EVENT_MEDIA_UPDATED = 'media-updated'
const EVENT_THUMB_UPDATED = 'thumbnail-updated'
const EVENT_DISCORD_STATUS = 'discord-status-changed'
const EVENT_NOTIFICATION_ACCESS = 'notification-access-changed'
// AndroidではRust側(JNI)がイベントでプッシュするためポーリングしない。デスクトップはADBの都合でポーリング。
const IS_ANDROID = typeof navigator !== 'undefined' && navigator.userAgent.includes('Android')

interface ThumbnailPayload {
  package_name: string
  title: string
  artist: string
  thumbnail_url: string
}

function App() {
  const [adbConnected, setAdbConnected] = createSignal(false)
  const [discordConnected, setDiscordConnected] = createSignal(false)
  const [media, setMedia] = createSignal<MediaInfo | null>(null)
  const [thumbUrl, setThumbUrl] = createSignal<string | null>(null)
  const [error, setError] = createSignal<string | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [lastFetch, setLastFetch] = createSignal<{ pos: number; time: number } | null>(null)
  const [now, setNow] = createSignal(Date.now())
  const [activeTab, setActiveTab] = createSignal<NavKey>('dashboard')
  const [navCollapsed, setNavCollapsed] = createSignal(false)
  const [drawerOpen, setDrawerOpen] = createSignal(false)
  const [notificationAccess, setNotificationAccess] = createSignal(true)

  const saved = !IS_ANDROID && typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_RPC_KEY) : null
  const [rpcEnabled, setRpcEnabled] = createSignal(saved === 'true')

  const [traySettings, setTraySettings] = createSignal<AppSettings>({
    auto_start: false,
    start_in_tray: true,
    minimize_to_tray: true,
    close_to_tray: true,
    media_whitelist: [],
    media_notification: false,
  })

  let pollingTimer: ReturnType<typeof setInterval> | undefined
  let lastPresenceKey: string | null = null
  let settingsLoaded = false
  let dragStart: { x: number; y: number } | null = null
  const [dragOffset, setDragOffset] = createSignal<number | null>(null)

  const displayPosition = createMemo(() => {
    const m = media()
    if (!m || m.position === null) return null
    const lf = lastFetch()
    if (!lf) return m.position
    if (!m.is_playing) return lf.pos
    const elapsed = now() - lf.time
    return lf.pos + elapsed
  })

  async function checkStatus(): Promise<boolean> {
    try {
      const [adb, dc] = await Promise.all([
        invoke<boolean>('get_adb_status'),
        invoke<boolean>('get_discord_status'),
      ])
      setAdbConnected(adb)
      setDiscordConnected(dc)
      return dc
    } catch (e) {
      console.error('status check failed', e)
      return false
    }
  }

  async function fetchMediaInfo() {
    setLoading(true)
    try {
      const result = await invoke<MediaInfo>('get_media_info')
      setMedia(result)
      // ponytail: 情報とサムネ完全分離
      setThumbUrl(result.thumbnail_url ?? null)
      setError(null)
      setAdbConnected(true)
      if (result.position !== null) {
        setLastFetch({ pos: result.position, time: Date.now() })
      } else {
        setLastFetch(null)
      }
      if (rpcEnabled()) {
        await invoke('connect_discord')
        const key = `${result.title}|${result.artist}|${result.album}|${result.is_playing}|${result.position ?? 0}`
        // ponytail: 切断中に取りこぼした presence を救うため、未接続観測時はキー一致でも送り直す
        if (key !== lastPresenceKey || !discordConnected()) {
          await invoke('update_discord_presence', { info: result })
          lastPresenceKey = key
        }
      }
    } catch (e) {
      const msg = String(e)
      // ponytail: 無音は正常なので切断しない。異常時のみ後片付けする
      const silent = msg.startsWith('NO_SESSION:')
      setMedia(null)
      setThumbUrl(null)
      setLastFetch(null)
      setError(silent ? null : msg)
      lastPresenceKey = null
      if (rpcEnabled() && !silent) {
        try {
          await invoke('disconnect_discord')
        } catch (err) {
          console.error('disconnect on error failed', err)
        }
      }
    } finally {
      setLoading(false)
    }
  }

  async function handleRpcChange(enabled: boolean) {
    setRpcEnabled(enabled)
    if (IS_ANDROID) {
      // Androidでは状態はKotlin側(SharedPreferences)が真実。通知ボタンと共有する。
      try {
        await invoke('set_rpc_enabled', { enabled })
      } catch (e) {
        console.error('rpc toggle failed', e)
      }
      return
    }
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

  async function handleReconnect() {
    if (IS_ANDROID) return
    try {
      await invoke('disconnect_discord')
      await invoke('connect_discord')
      lastPresenceKey = null
      const current = media()
      if (rpcEnabled() && current) {
        await invoke('update_discord_presence', { info: current })
        lastPresenceKey = `${current.title}|${current.artist}|${current.album}|${current.is_playing}|${current.position ?? 0}`
      }
      await checkStatus()
    } catch (e) {
      console.error('rpc reconnect failed', e)
    }
  }

  function onTouchStart(e: TouchEvent) {
    if (!IS_ANDROID) return
    const t = e.changedTouches[0]
    if (drawerOpen() || t.clientX < window.innerWidth / 2) {
      dragStart = { x: t.clientX, y: t.clientY }
    }
  }

  function onTouchMove(e: TouchEvent) {
    if (!dragStart) return
    const t = e.touches[0]
    const dx = t.clientX - dragStart.x
    const dy = t.clientY - dragStart.y
    if (dragOffset() === null && Math.abs(dx) <= 20) return
    if (Math.abs(dx) > Math.abs(dy)) {
      e.preventDefault()
      setDragOffset(dx)
    }
  }

  function onTouchEnd() {
    if (!dragStart) return
    const dx = dragOffset() ?? 0
    const startX = dragStart.x
    dragStart = null
    setDragOffset(null)
    if (dx === 0) return
    const THRESHOLD = 100
    if (drawerOpen()) {
      if (dx < -THRESHOLD) setDrawerOpen(false)
    } else if (dx > THRESHOLD && startX < window.innerWidth / 2) {
      setDrawerOpen(true)
    }
  }

  async function loadSettings() {
    try {
      const s = await invoke<AppSettings>('get_settings')
      setTraySettings(s)
      settingsLoaded = true
    } catch (e) {
      console.error('failed to load settings', e)
    }
  }

  async function updateSetting(key: keyof AppSettings, value: boolean | string | string[] | null) {
    const next = { ...traySettings(), [key]: value }
    setTraySettings(next)
    if (!settingsLoaded) return
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

    if (IS_ANDROID) {
      try {
        setRpcEnabled(await invoke<boolean>('get_rpc_enabled'))
      } catch (e) {
        console.error('failed to load rpc enabled', e)
      }
    }

    await fetchMediaInfo()
    if (!IS_ANDROID) {
      await checkStatus()
    }

    if (!IS_ANDROID) {
      check().then(update => {
        if (update && Notification.permission === 'granted') {
          new Notification('WSA RPC Bridge', {
            body: t('updates.update_available_notification', { version: update.version })
          })
        }
      }).catch(() => {})
    }

    if (IS_ANDROID) {
      const unlistenAccess = await listen<boolean>(EVENT_NOTIFICATION_ACCESS, (event) => {
        setNotificationAccess(event.payload)
      })
      onCleanup(unlistenAccess)
      invoke<boolean>('get_notification_access_status').then(setNotificationAccess).catch(() => {})

      const unlistenMedia = await listen<MediaInfo>(EVENT_MEDIA_UPDATED, async (event) => {
        const result = event.payload
        const prev = media()
        setMedia(result)
        setError(null)
        // ponytail: 情報とサムネ完全分離 — サムネ到着では lastFetch を更新しない
        if (result.position !== null) {
          const posChanged = prev?.position !== result.position
          const trackChanged = !prev || prev.title !== result.title || prev.artist !== result.artist || prev.package_name !== result.package_name
          const playChanged = prev?.is_playing !== result.is_playing
          if (posChanged || trackChanged || playChanged) {
            setLastFetch({ pos: result.position, time: Date.now() })
          }
        } else {
          setLastFetch(null)
        }
        // トラック切替で旧サムネをクリア、thumbnail-updated 到着まで placeholder
        if (!prev || prev.title !== result.title || prev.artist !== result.artist || prev.package_name !== result.package_name) {
          setThumbUrl(null)
        }
        if (!result.title) {
          setThumbUrl(null)
          lastPresenceKey = null
        }
        // AndroidのDiscordはRust側でサムネ後に送るためフロントからは送らない
        if (!IS_ANDROID && result.title && rpcEnabled()) {
          await invoke('connect_discord')
          const key = `${result.title}|${result.artist}|${result.album}|${result.is_playing}|${result.position ?? 0}`
          if (key !== lastPresenceKey) {
            await invoke('update_discord_presence', { info: result })
            lastPresenceKey = key
          }
        }
      })
      onCleanup(unlistenMedia)

      const unlistenThumb = await listen<ThumbnailPayload>(EVENT_THUMB_UPDATED, (event) => {
        const p = event.payload
        const cur = media()
        // stale なサムネは破棄
        if (!cur || cur.title !== p.title || cur.artist !== p.artist || cur.package_name !== p.package_name) return
        setThumbUrl(p.thumbnail_url)
      })
      onCleanup(unlistenThumb)

      const unlistenDiscord = await listen<boolean>(EVENT_DISCORD_STATUS, (event) => {
        setDiscordConnected(event.payload)
      })
      onCleanup(unlistenDiscord)

      const unlistenRpc = await listen<boolean>('rpc-enabled-changed', (event) => {
        setRpcEnabled(event.payload)
      })
      onCleanup(unlistenRpc)
    } else {
      pollingTimer = setInterval(async () => {
        const dc = await checkStatus()
        // ponytail: メディア有無と独立に再接続。無音中に Discord が起動しても追いつく
        if (rpcEnabled() && !dc) {
          try {
            await invoke('connect_discord')
          } catch (e) {
            console.error('periodic connect_discord failed', e)
          }
        }
        await fetchMediaInfo()
      }, POLL_INTERVAL)
      onCleanup(() => {
        if (pollingTimer) clearInterval(pollingTimer)
      })
    }

    if (rpcEnabled()) {
      try {
        await invoke('connect_discord')
      } catch (e) {
        console.error('initial connect_discord failed', e)
      }
    }
  })

  return (
    <div id="app" class={`shell ${IS_ANDROID ? 'android' : ''}`} onTouchStart={onTouchStart} onTouchMove={onTouchMove} onTouchEnd={onTouchEnd} onTouchCancel={onTouchEnd}>
      <Show when={IS_ANDROID}>
        <header class="top-bar">
          <button
            class="nav-toggle"
            onClick={() => setDrawerOpen(true)}
            title={t('nav.expand')}
            aria-label={t('nav.expand')}
          >
            <Menu size={22} />
          </button>
          <span class="sidebar-title">WSA RPC Bridge</span>
        </header>
      </Show>

      <Show when={IS_ANDROID && activeTab() === 'dashboard' && !notificationAccess()}>
        <div class="perm-banner">
          <span class="perm-banner-text">{t('permissions.notification_access_required')}</span>
          <button class="btn" onClick={() => invoke('open_notification_access_settings')}>
            {t('permissions.open_settings')}
          </button>
        </div>
      </Show>

      <Sidebar
        active={activeTab()}
        collapsed={navCollapsed()}
        android={IS_ANDROID}
        open={drawerOpen()}
        dragOffset={dragOffset()}
        onSelect={(key) => {
          setActiveTab(key)
          setDrawerOpen(false)
        }}
        onClose={() => setDrawerOpen(false)}
        onToggleCollapsed={() => setNavCollapsed((c) => !c)}
      />

      <main class="content">
        <Show when={activeTab() === 'dashboard'}>
          <Dashboard
            media={media()}
            thumbnailUrl={thumbUrl()}
            loading={loading()}
            error={error()}
            displayPosition={displayPosition()}
            adbConnected={adbConnected()}
            discordConnected={discordConnected()}
            rpcEnabled={rpcEnabled()}
            android={IS_ANDROID}
            whitelistEmpty={traySettings().media_whitelist.length === 0}
            onReconnect={handleReconnect}
            onRetry={fetchMediaInfo}
            onOpenSettings={() => setActiveTab('settings')}
          />
        </Show>

        <Show when={activeTab() === 'settings'}>
          <SettingsPanel
            rpcEnabled={rpcEnabled()}
            onRpcChange={handleRpcChange}
            traySettings={traySettings()}
            onUpdateSetting={updateSetting}
            isAndroid={IS_ANDROID}
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