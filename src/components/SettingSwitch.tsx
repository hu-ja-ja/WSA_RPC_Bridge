import { Switch } from '@kobalte/core/switch'
import { Show } from 'solid-js'

interface SettingSwitchProps {
  checked: boolean
  onChange: (checked: boolean) => void
  label: string
  description?: string
}

export function SettingSwitch(props: SettingSwitchProps) {
  return (
    <Switch
      checked={props.checked}
      onChange={props.onChange}
      class="rpc-switch"
    >
      <Switch.Label class="switch-label">
        <span>{props.label}</span>
        <Show when={props.description}>
          <span class="switch-desc">{props.description}</span>
        </Show>
      </Switch.Label>
      <Switch.Control class="switch-track">
        <Switch.Thumb class="switch-thumb" />
      </Switch.Control>
    </Switch>
  )
}