import { Show } from 'solid-js'
import { Smartphone } from 'lucide-solid'
import { IconBrandDiscord } from '@tabler/icons-solidjs'
import { MediaCard } from './MediaCard'
import { t } from '../i18n'

interface MediaInfo {
  title: string
  artist: string
  album: string
  package_name: string
  thumbnail_url: string | null
  position: number | null
  duration: number | null
  is_playing: boolean
}

interface DashboardProps {
  media: MediaInfo | null
  loading: boolean
  error: string | null
  displayPosition: number | null
  adbConnected: boolean
  discordConnected: boolean
  rpcEnabled: boolean
  android?: boolean
  onRetry: () => void
}

function StatusCard(props: {
  icon: typeof Smartphone
  label: string
  connected: boolean
  status: string
}) {
  return (
    <div class={`status-card ${props.connected ? 'connected' : ''}`}>
      <props.icon class="status-card-icon" size={18} />
      <div class="status-card-body">
        <span class="status-card-label">{props.label}</span>
        <span class="status-card-value">
          <span class={`dot ${props.connected ? 'connected' : 'disconnected'}`} />
          {props.status}
        </span>
      </div>
    </div>
  )
}

export function Dashboard(props: DashboardProps) {
  return (
    <div class="dashboard">
      <section class="status-grid">
        <Show when={!props.android}>
          <StatusCard
            icon={Smartphone}
            label="ADB"
            connected={props.adbConnected}
            status={props.adbConnected ? t('status.connected') : t('status.disconnected')}
          />
        </Show>
        <StatusCard
          icon={IconBrandDiscord}
          label="Discord RPC"
          connected={props.discordConnected}
          status={
            props.discordConnected
              ? t('status.connected')
              : props.rpcEnabled
                ? t('status.waiting')
                : t('status.disconnected')
          }
        />
      </section>

      <section class="dashboard-now">
        <h2 class="page-heading">{t('dashboard.now_playing')}</h2>
        <MediaCard
          media={props.media}
          loading={props.loading}
          error={props.error}
          displayPosition={props.displayPosition}
          onRetry={props.onRetry}
        />
      </section>
    </div>
  )
}