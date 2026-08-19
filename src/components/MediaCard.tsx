import { Show } from 'solid-js'
import { RefreshCw, Settings } from 'lucide-solid'
import { t } from '../i18n'

const PLACEHOLDER_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><rect width="100" height="100" rx="8" fill="#e5e4e7"/><g fill="none" stroke="#9ca3af" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><circle cx="36" cy="66" r="10"/><circle cx="66" cy="58" r="10"/><path d="M46 66 V30 L76 22 V58"/></g></svg>`
const NOART_URI = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(PLACEHOLDER_SVG)}`

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

interface MediaCardProps {
  media: MediaInfo | null
  loading: boolean
  error: string | null
  displayPosition: number | null
  android?: boolean
  whitelistEmpty?: boolean
  onRetry: () => void
  onOpenSettings?: () => void
}

function formatTime(ms: number): string {
  const totalSec = Math.floor(ms / 1000)
  const min = Math.floor(totalSec / 60)
  const sec = totalSec % 60
  return `${min}:${sec.toString().padStart(2, '0')}`
}

export function MediaCard(props: MediaCardProps) {
  return (
    <Show when={props.loading && !props.media} fallback={
      <Show when={props.media?.title ? props.media : null} fallback={
        <div class="empty-state">
          <p>{props.error ?? (props.android ? t("media.none_android") : t("media.none"))}</p>
          <Show when={props.android && props.whitelistEmpty}>
            <p class="empty-hint">{t("media.none_android_setup")}</p>
            <button onClick={props.onOpenSettings} class="btn"><Settings size={16} />{t("media.configure_detection")}</button>
          </Show>
          <Show when={!props.android}>
            <button onClick={props.onRetry} class="btn"><RefreshCw size={16} />{t("media.retry")}</button>
          </Show>
        </div>
      }>
        {(m) => (
          <div class="media-card">
            <Show when={m().thumbnail_url}>
              <img
                src={m().thumbnail_url!}
                alt="album art"
                class="thumb"
                onerror={(e) => {
                  const img = e.currentTarget
                  if (img.src !== NOART_URI) {
                    img.src = NOART_URI
                  }
                }}
              />
            </Show>
            <div class="media-body">
              <div class="track-title">
                <span class={`play-icon ${m().is_playing ? 'playing' : 'paused'}`}>
                  {m().is_playing ? '▶' : '⏸'}
                </span>
                {m().title}
              </div>
              <div class="artist">{m().artist}</div>
              <div class="album">{m().album}</div>
              <div class="position">
                <Show when={props.displayPosition !== null}>
                  <span>{formatTime(props.displayPosition!)}</span>
                </Show>
                <Show when={m().duration !== null}>
                  <span> / {formatTime(m().duration!)}</span>
                </Show>
              </div>
            </div>
          </div>
        )}
      </Show>
    }>
      <div class="loading-msg">{t("media.loading")}</div>
    </Show>
  )
}
