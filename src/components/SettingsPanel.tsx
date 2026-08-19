import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { LoaderCircle } from 'lucide-solid'
import { SettingSwitch } from './SettingSwitch'
import { t } from '../i18n'

interface AppSettings {
  auto_start: boolean
  start_in_tray: boolean
  minimize_to_tray: boolean
  close_to_tray: boolean
  media_whitelist: string[]
  media_notification: boolean
}

interface MediaApp {
  label: string
  pkg: string
}

interface SettingsPanelProps {
  rpcEnabled: boolean
  onRpcChange: (enabled: boolean) => void
  traySettings: AppSettings
  onUpdateSetting: (key: keyof AppSettings, value: boolean | string | string[] | null) => void
  isAndroid: boolean
}

let cached: MediaApp[] | null = null
let inflight: Promise<MediaApp[]> | null = null

async function fetchApps(): Promise<MediaApp[]> {
  if (cached) return cached
  if (!inflight) {
    inflight = (async () => {
      try {
        const list = await invoke<string[]>('list_media_apps')
        cached = list.map((s) => {
          const [label, pkg] = s.split('\t')
          return { label, pkg }
        })
        return cached
      } catch (e) {
        inflight = null
        throw e
      }
    })()
  }
  return inflight
}

export function SettingsPanel(props: SettingsPanelProps) {
  const [apps, setApps] = createSignal<MediaApp[]>([])
  const [query, setQuery] = createSignal('')
  const [appsLoaded, setAppsLoaded] = createSignal(false)
  const [appsLoading, setAppsLoading] = createSignal(false)

  onMount(() => {
    if (!props.isAndroid) return
    // 遅延ロード: JNIのアプリ列挙は重いため、初回描画の後に非同期で取得する
    const timer = setTimeout(async () => {
      setAppsLoading(true)
      try {
        setApps(await fetchApps())
      } catch (e) {
        console.error('failed to load media apps', e)
      } finally {
        setAppsLoading(false)
        setAppsLoaded(true)
      }
    }, 100)
    onCleanup(() => clearTimeout(timer))
  })

  const filtered = createMemo(() => {
    const q = query().trim().toLowerCase()
    if (!q) return apps()
    return apps().filter((a) => a.label.toLowerCase().includes(q) || a.pkg.includes(q))
  })

  function toggle(pkg: string, checked: boolean) {
    const current = props.traySettings.media_whitelist
    const next = checked ? [...current, pkg] : current.filter((p) => p !== pkg)
    props.onUpdateSetting('media_whitelist', next)
  }

  return (
    <div class="settings-panel">
      <h2 class="page-heading">{t("nav.settings")}</h2>

      <div class="settings-card">
        <h3 class="card-heading">{t("settings.rpc_title")}</h3>
        <SettingSwitch
          checked={props.rpcEnabled}
          onChange={props.onRpcChange}
          label={t("settings.enable_rpc")}
          description={t("settings.rpc_description")}
        />
      </div>

      <Show when={props.isAndroid}>
        <div class="settings-card">
          <h3 class="card-heading">{t("settings.media_notification_title")}</h3>
          <SettingSwitch
            checked={props.traySettings.media_notification}
            onChange={(v) => props.onUpdateSetting('media_notification', v)}
            label={t("settings.media_notification_title")}
            description={t("settings.media_notification_description")}
          />
        </div>

        <div class="settings-card">
          <h3 class="card-heading">{t("settings.media_whitelist_title")}</h3>
          <p class="switch-desc">{t("settings.media_whitelist_description")}</p>
          <Show when={props.traySettings.media_whitelist.length === 0}>
            <p class="whitelist-empty">{t("settings.media_whitelist_empty")}</p>
          </Show>
          <input
            class="whitelist-search"
            type="text"
            placeholder={t("settings.media_search_placeholder")}
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
          />
          <Show when={appsLoading()}>
            <div class="whitelist-loading">
              <LoaderCircle class="spinner" size={22} />
              <span>{t('media.loading')}</span>
            </div>
          </Show>
          <Show when={appsLoaded() && !appsLoading() && filtered().length === 0}>
            <p class="whitelist-no-matches">{t("settings.media_no_matches")}</p>
          </Show>
          <Show when={appsLoaded() && !appsLoading() && filtered().length > 0}>
            <ul class="whitelist-list">
              <For each={filtered()}>
                {(app) => (
                  <li>
                    <label class="whitelist-item">
                      <input
                        type="checkbox"
                        checked={props.traySettings.media_whitelist.includes(app.pkg)}
                        onChange={(e) => toggle(app.pkg, e.currentTarget.checked)}
                      />
                      <span class="whitelist-text">
                        <span class="whitelist-label">{app.label}</span>
                        <span class="whitelist-pkg">{app.pkg}</span>
                      </span>
                    </label>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </div>
      </Show>

      <Show when={!props.isAndroid}>
        <div class="settings-card">
          <h3 class="card-heading">{t("settings.tray_title")}</h3>

          <SettingSwitch
            checked={props.traySettings.auto_start}
            onChange={(v) => props.onUpdateSetting('auto_start', v)}
            label={t("settings.auto_start")}
            description={t("settings.auto_start_description")}
          />

          <div class="setting-sep" />

          <SettingSwitch
            checked={props.traySettings.start_in_tray}
            onChange={(v) => props.onUpdateSetting('start_in_tray', v)}
            label={t("settings.start_in_tray")}
            description={t("settings.start_in_tray_description")}
          />

          <div class="setting-sep" />

          <SettingSwitch
            checked={props.traySettings.minimize_to_tray}
            onChange={(v) => props.onUpdateSetting('minimize_to_tray', v)}
            label={t("settings.minimize_to_tray")}
            description={t("settings.minimize_to_tray_description")}
          />

          <div class="setting-sep" />

          <SettingSwitch
            checked={props.traySettings.close_to_tray}
            onChange={(v) => props.onUpdateSetting('close_to_tray', v)}
            label={t("settings.close_to_tray")}
            description={t("settings.close_to_tray_description")}
          />
        </div>
      </Show>
    </div>
  )
}
