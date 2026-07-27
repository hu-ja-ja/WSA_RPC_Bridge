import { t } from '../i18n'

interface StatusBarProps {
  adbConnected: boolean
  discordConnected: boolean
  rpcEnabled: boolean
}

export function StatusBar(props: StatusBarProps) {
  return (
    <div id="status-bar">
      <span class={`dot ${props.adbConnected ? 'connected' : 'disconnected'}`} />
      <span>ADB</span>
      <span class="status-value">{props.adbConnected ? t("status.connected") : t("status.disconnected")}</span>

      <span class="status-sep">|</span>

      <span class={`dot ${props.discordConnected ? 'connected' : 'disconnected'}`} />
      <span>Discord RPC</span>
      <span class="status-value">
        {props.discordConnected ? t("status.connected") : props.rpcEnabled ? t("status.waiting") : t("status.disconnected")}
      </span>
    </div>
  )
}
