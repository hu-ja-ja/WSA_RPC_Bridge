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
      <span class="status-value">{props.adbConnected ? '接続済み' : '切断'}</span>

      <span class="status-sep">|</span>

      <span class={`dot ${props.discordConnected ? 'connected' : 'disconnected'}`} />
      <span>Discord RPC</span>
      <span class="status-value">
        {props.discordConnected ? '接続済み' : props.rpcEnabled ? '待機中' : '切断'}
      </span>
    </div>
  )
}
