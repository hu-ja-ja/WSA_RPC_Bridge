import { Show } from 'solid-js'

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
  onRetry: () => void
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
      <Show when={props.media} fallback={
        <div class="empty-state">
          <p>{props.error ?? '再生中のメディアはありません'}</p>
          <button onClick={props.onRetry} class="btn">再試行</button>
        </div>
      }>
        {(m) => (
          <div class="media-card">
            <Show when={m().thumbnail_url}>
              <img src={m().thumbnail_url!} alt="album art" class="thumb" />
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
      <div class="loading-msg">読み込み中...</div>
    </Show>
  )
}
