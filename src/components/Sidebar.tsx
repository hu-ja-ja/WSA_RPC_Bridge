import { For } from 'solid-js'
import type { Component } from 'solid-js'
import { LayoutDashboard, Settings, CloudDownload, Scale, Info, PanelLeftClose, PanelLeftOpen } from 'lucide-solid'
import { t } from '../i18n'

type NavKey = 'dashboard' | 'settings' | 'updates' | 'licenses' | 'about'

type NavLabelKey =
  | 'app.nav.dashboard'
  | 'app.nav.settings'
  | 'app.nav.updates'
  | 'app.nav.licenses'
  | 'app.nav.about'

interface NavItem {
  key: NavKey
  labelKey: NavLabelKey
  icon: Component<{ size?: number | string; class?: string }>
}

interface SidebarProps {
  active: NavKey
  collapsed: boolean
  onSelect: (key: NavKey) => void
  onToggleCollapsed: () => void
}

const mainItems: NavItem[] = [
  { key: 'dashboard', labelKey: 'app.nav.dashboard', icon: LayoutDashboard },
  { key: 'settings', labelKey: 'app.nav.settings', icon: Settings },
  { key: 'updates', labelKey: 'app.nav.updates', icon: CloudDownload },
]

const footerItems: NavItem[] = [
  { key: 'licenses', labelKey: 'app.nav.licenses', icon: Scale },
  { key: 'about', labelKey: 'app.nav.about', icon: Info },
]

export function Sidebar(props: SidebarProps) {
  return (
    <nav class={`sidebar ${props.collapsed ? 'collapsed' : ''}`} aria-label="Main navigation">
      <div class="sidebar-header">
        {props.collapsed ? (
          <button class="nav-toggle" onClick={props.onToggleCollapsed} title={t('app.nav.expand')} aria-label={t('app.nav.expand')}>
            <PanelLeftOpen size={18} />
          </button>
        ) : (
          <button class="nav-toggle" onClick={props.onToggleCollapsed} title={t('app.nav.collapse')} aria-label={t('app.nav.collapse')}>
            <PanelLeftClose size={18} />
          </button>
        )}
        <span class="sidebar-title">WSA RPC Bridge</span>
      </div>

      <div class="sidebar-body">
        <ul class="nav-group">
          <For each={mainItems}>
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
  )
}

export type { NavKey }
