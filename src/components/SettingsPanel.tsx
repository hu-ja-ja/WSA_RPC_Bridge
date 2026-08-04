import { SettingSwitch } from './SettingSwitch'
import { t } from '../i18n'

interface AppSettings {
  auto_start: boolean
  start_in_tray: boolean
  minimize_to_tray: boolean
  close_to_tray: boolean
}

interface SettingsPanelProps {
  rpcEnabled: boolean
  onRpcChange: (enabled: boolean) => void
  traySettings: AppSettings
  onUpdateSetting: (key: keyof AppSettings, value: boolean | string | null) => void
}

export function SettingsPanel(props: SettingsPanelProps) {
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
    </div>
  )
}