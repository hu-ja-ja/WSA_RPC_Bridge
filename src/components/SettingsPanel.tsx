import { createSignal, Show } from 'solid-js'
import { open } from '@tauri-apps/plugin-dialog'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { SettingSwitch } from './SettingSwitch'
import { t } from '../i18n'

interface AppSettings {
  auto_start: boolean
  start_in_tray: boolean
  minimize_to_tray: boolean
  close_to_tray: boolean
  thumbnail_cache_enabled: boolean
  thumbnail_cache_path: string | null
}

interface SettingsPanelProps {
  rpcEnabled: boolean
  onRpcChange: (enabled: boolean) => void
  traySettings: AppSettings
  onUpdateSetting: (key: keyof AppSettings, value: boolean | string | null) => void
  defaultCachePath: string
}

type UpdateState = 'idle' | 'checking' | 'uptodate' | 'available' | 'downloading'

export function SettingsPanel(props: SettingsPanelProps) {
  const [updateState, setUpdateState] = createSignal<UpdateState>('idle')
  const [updateVersion, setUpdateVersion] = createSignal<string | null>(null)
  const [downloadProgress, setDownloadProgress] = createSignal(0)
  const [updateError, setUpdateError] = createSignal<string | null>(null)

  let pendingUpdate: any = null

  const effectiveCachePath = (): string => {
    return props.traySettings.thumbnail_cache_path || props.defaultCachePath
  }

  const handleBrowse = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.thumbnail_cache_path'),
    })
    if (selected) {
      props.onUpdateSetting('thumbnail_cache_path', selected)
    }
  }

  async function handleCheckUpdate() {
    setUpdateState('checking')
    setUpdateError(null)
    try {
      pendingUpdate = await check()
      if (!pendingUpdate) {
        setUpdateState('uptodate')
        return
      }
      setUpdateVersion(pendingUpdate.version)
      setUpdateState('available')
    } catch (e) {
      setUpdateError(String(e))
      setUpdateState('idle')
    }
  }

  async function handleInstall() {
    if (!pendingUpdate) return
    setUpdateState('downloading')
    setDownloadProgress(0)
    try {
      let downloaded = 0
      let contentLength = 0
      await pendingUpdate.downloadAndInstall((event: any) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength
            break
          case 'Progress':
            downloaded += event.data.chunkLength
            if (contentLength > 0) {
              setDownloadProgress(Math.round((downloaded / contentLength) * 100))
            }
            break
        }
      })
      await relaunch()
    } catch (e) {
      setUpdateError(String(e))
      setUpdateState('available')
    }
  }

  return (
    <>
      <div class="settings-card">
        <SettingSwitch
          checked={props.rpcEnabled}
          onChange={props.onRpcChange}
          label={t("settings.enable_rpc")}
        />
        <p class="switch-desc">
          {t("settings.rpc_description")}
        </p>
      </div>

      <div class="settings-divider" />

      <h3 class="section-title">{t("settings.tray_title")}</h3>

      <div class="settings-card">
        <SettingSwitch
          checked={props.traySettings.auto_start}
          onChange={(v) => props.onUpdateSetting('auto_start', v)}
          label={t("settings.auto_start")}
        />

        <SettingSwitch
          checked={props.traySettings.start_in_tray}
          onChange={(v) => props.onUpdateSetting('start_in_tray', v)}
          label={t("settings.start_in_tray")}
        />

        <SettingSwitch
          checked={props.traySettings.minimize_to_tray}
          onChange={(v) => props.onUpdateSetting('minimize_to_tray', v)}
          label={t("settings.minimize_to_tray")}
        />

        <SettingSwitch
          checked={props.traySettings.close_to_tray}
          onChange={(v) => props.onUpdateSetting('close_to_tray', v)}
          label={t("settings.close_to_tray")}
        />
      </div>

      <div class="settings-divider" />

      <h3 class="section-title">{t("settings.thumbnail_cache_title")}</h3>

      <div class="settings-card">
        <SettingSwitch
          checked={props.traySettings.thumbnail_cache_enabled}
          onChange={(v) => props.onUpdateSetting('thumbnail_cache_enabled', v)}
          label={t("settings.thumbnail_cache_enabled")}
        />
        <p class="switch-desc">
          {t("settings.thumbnail_cache_description")}
        </p>

        <Show when={props.traySettings.thumbnail_cache_enabled}>
          <div class="cache-path-row">
            <input
              type="text"
              class="cache-path-input"
              value={effectiveCachePath()}
              readOnly
            />
            <button onClick={handleBrowse} class="btn-browse">
              {t("settings.browse")}
            </button>
          </div>
        </Show>
      </div>

      <div class="settings-divider" />

      <h3 class="section-title">{t("settings.updates_title")}</h3>

      <div class="settings-card">
        <Show when={updateState() === 'idle' || updateState() === 'available'}>
          <button onClick={handleCheckUpdate} class="btn-update" disabled={updateState() === 'checking'}>
            {updateState() === 'idle' ? t("settings.check_update") : t("settings.check_update_again")}
          </button>
        </Show>

        <Show when={updateState() === 'checking'}>
          <p class="update-status">{t("settings.checking")}</p>
        </Show>

        <Show when={updateState() === 'uptodate'}>
          <p class="update-status update-ok">{t("settings.up_to_date")}</p>
        </Show>

        <Show when={updateState() === 'available' && updateVersion()}>
          <div class="update-info">
            <p class="update-status update-available">
              {t("settings.update_available", { version: updateVersion()! })}
            </p>
            <button onClick={handleInstall} class="btn-install">
              {t("settings.install_update")}
            </button>
          </div>
        </Show>

        <Show when={updateState() === 'downloading'}>
          <div class="update-progress">
            <p class="update-status">{t("settings.download_progress", { progress: downloadProgress() })}</p>
            <div class="progress-bar">
              <div class="progress-bar-fill" style={{ width: `${downloadProgress()}%` }} />
            </div>
          </div>
        </Show>

        <Show when={updateError()}>
          <p class="update-error">{updateError()}</p>
        </Show>
      </div>
    </>
  )
}
