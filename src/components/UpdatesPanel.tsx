import { createSignal, Show } from 'solid-js'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { openUrl } from '@tauri-apps/plugin-opener'
import { t } from '../i18n'

type UpdateState = 'idle' | 'checking' | 'uptodate' | 'available' | 'downloading' | 'error'

const APP_VERSION = 'v0.2.0'

const repoUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge'
const changelogUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/CHANGELOG.md'

export function UpdatesPanel() {
  const [updateState, setUpdateState] = createSignal<UpdateState>('idle')
  const [updateVersion, setUpdateVersion] = createSignal<string | null>(null)
  const [downloadProgress, setDownloadProgress] = createSignal(0)
  const [updateError, setUpdateError] = createSignal<string | null>(null)

  let pendingUpdate: any = null

  async function handleCheckUpdate() {
    setUpdateState('checking')
    setUpdateError(null)
    try {
      pendingUpdate = await check()
      if (!pendingUpdate) {
        setUpdateState('uptodate')
        return
      }
      setUpdateVersion(pendingUpdate.version)
      setUpdateState('available')
    } catch (e) {
      setUpdateError(String(e))
      setUpdateState('error')
    }
  }

  async function handleInstall() {
    if (!pendingUpdate) return
    setUpdateState('downloading')
    setDownloadProgress(0)
    try {
      let downloaded = 0
      let contentLength = 0
      await pendingUpdate.downloadAndInstall((event: any) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength
            break
          case 'Progress':
            downloaded += event.data.chunkLength
            if (contentLength > 0) {
              setDownloadProgress(Math.round((downloaded / contentLength) * 100))
            }
            break
        }
      })
      await relaunch()
    } catch (e) {
      setUpdateError(String(e))
      setUpdateState('available')
    }
  }

  return (
    <div class="settings-panel">
      <h2 class="page-heading">{t("app.nav.updates")}</h2>

      <div class="settings-card">
        <h3 class="card-heading">{t("settings.updates_title")}</h3>

        <div class="update-row">
          <div class="update-meta">
            <span class="update-meta-label">{t("updates.installed_label")}</span>
            <span class="update-version-tag">{APP_VERSION}</span>
            <p class="switch-desc">{t("updates.current_description")}</p>
          </div>

          <div class="update-action">
            <Show when={updateState() === 'downloading'}>
              <div class="update-progress">
                <p class="update-status">{t("settings.download_progress", { progress: downloadProgress() })}</p>
                <div class="progress-bar">
                  <div class="progress-bar-fill" style={{ width: `${downloadProgress()}%` }} />
                </div>
              </div>
            </Show>

            <Show when={updateState() === 'available'}>
              <p class="update-status update-available">
                {t("settings.update_available", { version: updateVersion()! })}
              </p>
              <button onClick={handleInstall} class="btn-install">
                {t("settings.install_update")}
              </button>
            </Show>

            <Show when={updateState() === 'uptodate'}>
              <p class="update-status update-ok">{t("settings.up_to_date")}</p>
              <button onClick={handleCheckUpdate} class="btn-update">
                {t("settings.check_update_again")}
              </button>
            </Show>

            <Show when={updateState() === 'checking'}>
              <p class="update-status">{t("settings.checking")}</p>
              <button class="btn-update" disabled>
                {t("settings.check_update")}
              </button>
            </Show>

            <Show when={updateState() === 'idle' || updateState() === 'error'}>
              <Show when={updateError()}>
                <p class="update-error">{updateError()}</p>
              </Show>
              <button onClick={handleCheckUpdate} class="btn-update">
                {updateState() === 'error' ? t("settings.check_update_again") : t("settings.check_update")}
              </button>
            </Show>
          </div>
        </div>

        <div class="setting-sep" />

        <div class="link-row">
          <button class="link-button" onClick={() => openUrl(repoUrl)}>
            {t("updates.repository")}
          </button>
          <button class="link-button" onClick={() => openUrl(changelogUrl)}>
            {t("updates.changelog")}
          </button>
        </div>
      </div>
    </div>
  )
}