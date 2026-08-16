import { For, Show } from 'solid-js'
import type { Component } from 'solid-js'
import { LayoutDashboard, Settings, CloudDownload, Scale, Info, PanelLeftClose, PanelLeftOpen, X } from 'lucide-solid'
import { t } from '../i18n'

type NavKey = 'dashboard' | 'settings' | 'updates' | 'licenses' | 'about'

type NavLabelKey =
  | 'nav.dashboard'
  | 'nav.settings'
  | 'nav.updates'
  | 'nav.licenses'
  | 'nav.about'

interface NavItem {
  key: NavKey
  labelKey: NavLabelKey
  icon: Component<{ size?: number | string; class?: string }>
}

interface SidebarProps {
  active: NavKey
  collapsed: boolean
  android?: boolean
  open?: boolean
  onSelect: (key: NavKey) => void
  onToggleCollapsed: () => void
  onClose?: () => void
}

const mainItems: NavItem[] = [
  { key: 'dashboard', labelKey: 'nav.dashboard', icon: LayoutDashboard },
  { key: 'settings', labelKey: 'nav.settings', icon: Settings },
  { key: 'updates', labelKey: 'nav.updates', icon: CloudDownload },
]

const footerItems: NavItem[] = [
  { key: 'licenses', labelKey: 'nav.licenses', icon: Scale },
  { key: 'about', labelKey: 'nav.about', icon: Info },
]

export function Sidebar(props: SidebarProps) {
  const items = () => (props.android ? mainItems.filter((i) => i.key !== 'updates') : mainItems)

  return (
    <>
      <nav class={`sidebar ${props.collapsed ? 'collapsed' : ''} ${props.android ? 'android' : ''} ${props.open ? 'open' : ''}`} aria-label="Main navigation">
        <div class="sidebar-header">
          {props.android ? (
            <button class="nav-toggle" onClick={props.onClose} title={t('nav.close')} aria-label={t('nav.close')}>
              <X size={20} />
            </button>
          ) : props.collapsed ? (
            <button class="nav-toggle" onClick={props.onToggleCollapsed} title={t('nav.expand')} aria-label={t('nav.expand')}>
              <PanelLeftOpen size={18} />
            </button>
          ) : (
            <button class="nav-toggle" onClick={props.onToggleCollapsed} title={t('nav.collapse')} aria-label={t('nav.collapse')}>
              <PanelLeftClose size={18} />
            </button>
          )}
          <span class="sidebar-title">WSA RPC Bridge</span>
        </div>

        <div class="sidebar-body">
          <ul class="nav-group">
            <For each={items()}>
              {(item) => (
                <li>
                  <button
                    class={`nav-item ${props.active === item.key ? 'selected' : ''}`}
                    onClick={() => props.onSelect(item.key)}
                    aria-current={props.active === item.key ? 'page' : undefined}
                    title={props.collapsed ? t(item.labelKey) : undefined}
                  >
                    <item.icon class="nav-icon" size={18} />
                    <span class="nav-label">{t(item.labelKey)}</span>
                  </button>
                </li>
              )}
            </For>
          </ul>

          <ul class="nav-group sidebar-footer">
            <For each={footerItems}>
              {(item) => (
                <li>
                  <button
                    class={`nav-item ${props.active === item.key ? 'selected' : ''}`}
                    onClick={() => props.onSelect(item.key)}
                    aria-current={props.active === item.key ? 'page' : undefined}
                    title={props.collapsed ? t(item.labelKey) : undefined}
                  >
                    <item.icon class="nav-icon" size={18} />
                    <span class="nav-label">{t(item.labelKey)}</span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </div>
      </nav>
      <Show when={props.android && props.open}>
        <div class="drawer-backdrop" onClick={props.onClose} />
      </Show>
    </>
  )
}

export type { NavKey }
