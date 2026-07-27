import { SettingSwitch } from './SettingSwitch'
import { t } from '../i18n'

interface AppSettings {
  start_in_tray: boolean
  minimize_to_tray: boolean
  close_to_tray: boolean
}

interface SettingsPanelProps {
  rpcEnabled: boolean
  onRpcChange: (enabled: boolean) => void
  traySettings: AppSettings
  onUpdateSetting: (key: keyof AppSettings, value: boolean) => void
}

export function SettingsPanel(props: SettingsPanelProps) {
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
    </>
  )
}
