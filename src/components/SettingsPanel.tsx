import { Show } from 'solid-js'
import { open } from '@tauri-apps/plugin-dialog'
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

export function SettingsPanel(props: SettingsPanelProps) {
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

  return (
    <div class="settings-panel">
      <h2 class="page-heading">{t("app.nav.settings")}</h2>

      <div class="settings-card">
        <h3 class="card-heading">{t("settings.rpc_title")}</h3>
        <SettingSwitch
          checked={props.rpcEnabled}
          onChange={props.onRpcChange}
          label={t("settings.enable_rpc")}
          description={t("settings.rpc_description")}
        />
      </div>

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

      <div class="settings-card">
        <h3 class="card-heading">{t("settings.thumbnail_cache_title")}</h3>

        <SettingSwitch
          checked={props.traySettings.thumbnail_cache_enabled}
          onChange={(v) => props.onUpdateSetting('thumbnail_cache_enabled', v)}
          label={t("settings.thumbnail_cache_enabled")}
          description={t("settings.thumbnail_cache_description")}
        />

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
    </div>
  )
}
