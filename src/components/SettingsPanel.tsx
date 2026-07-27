import { Show } from 'solid-js'
import { open } from '@tauri-apps/plugin-dialog'
import { SettingSwitch } from './SettingSwitch'
import { t } from '../i18n'

interface AppSettings {
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
    </>
  )
}
