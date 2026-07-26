import { Switch } from '@kobalte/core/switch'

interface SettingSwitchProps {
  checked: boolean
  onChange: (checked: boolean) => void
  label: string
}

export function SettingSwitch(props: SettingSwitchProps) {
  return (
    <Switch
      checked={props.checked}
      onChange={props.onChange}
      class="rpc-switch"
    >
      <Switch.Label class="switch-label">
        {props.label}
      </Switch.Label>
      <Switch.Control class="switch-track">
        <Switch.Thumb class="switch-thumb" />
      </Switch.Control>
    </Switch>
  )
}
