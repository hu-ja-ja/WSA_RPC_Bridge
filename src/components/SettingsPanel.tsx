import { SettingSwitch } from './SettingSwitch'

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
          label="Discord Rich Presence を有効にする"
        />
        <p class="switch-desc">
          再生中のメディア情報を Discord のアクティビティに表示します。
        </p>
      </div>

      <div class="settings-divider" />

      <h3 class="section-title">タスクトレイ設定</h3>

      <div class="settings-card">
        <SettingSwitch
          checked={props.traySettings.start_in_tray}
          onChange={(v) => props.onUpdateSetting('start_in_tray', v)}
          label="起動時にタスクトレイに格納"
        />

        <SettingSwitch
          checked={props.traySettings.minimize_to_tray}
          onChange={(v) => props.onUpdateSetting('minimize_to_tray', v)}
          label="最小化時にタスクトレイに収納"
        />

        <SettingSwitch
          checked={props.traySettings.close_to_tray}
          onChange={(v) => props.onUpdateSetting('close_to_tray', v)}
          label="閉じる時にタスクトレイに収納"
        />
      </div>
    </>
  )
}
