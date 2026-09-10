import { createSignal, createResource, Show } from 'solid-js'
import { check } from '@tauri-apps/plugin-updater'
import { getVersion } from '@tauri-apps/api/app'
import { relaunch } from '@tauri-apps/plugin-process'
import { openUrl } from '@tauri-apps/plugin-opener'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { IconBrandGithub } from '@tabler/icons-solidjs'
import { ScrollText } from 'lucide-solid'
import { t } from '../i18n'

type UpdateState = 'idle' | 'checking' | 'uptodate' | 'available' | 'downloading' | 'installing' | 'error'

// AndroidではRust側(JNI+PackageInstaller相当)が更新を担うためWin版updaterは使わない。
const IS_ANDROID = typeof navigator !== 'undefined' && navigator.userAgent.includes('Android')

interface AndroidUpdateStatus {
  currentVersion: string
  latestVersion: string
  available: boolean
  url: string | null
}

const [appVersion] = createResource(async () => `v${await getVersion()}`)

const repoUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge'
const changelogUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/CHANGELOG.md'

export function UpdatesPanel() {
  const [updateState, setUpdateState] = createSignal<UpdateState>('idle')
  const [updateVersion, setUpdateVersion] = createSignal<string | null>(null)
  const [downloadProgress, setDownloadProgress] = createSignal(0)
  const [updateError, setUpdateError] = createSignal<string | null>(null)
  const [permissionNeeded, setPermissionNeeded] = createSignal(false)
  const [autoAvailable, setAutoAvailable] = createSignal(false)

  let pendingUpdate: any = null
  let pendingApkUrl: string | null = null
  let pendingApkPath: string | null = null
  let pendingApkVersion: string | null = null

  async function handleCheckUpdate() {
    setUpdateState('checking')
    setUpdateError(null)
    setPermissionNeeded(false)
    setAutoAvailable(false)
    if (IS_ANDROID) {
      try {
        const status = await invoke<AndroidUpdateStatus>('check_android_update')
        if (!status.available) {
          setUpdateState('uptodate')
          return
        }
        pendingApkUrl = status.url
        if (pendingApkVersion !== status.latestVersion) {
          pendingApkPath = null
          pendingApkVersion = null
        }
        setAutoAvailable(!!status.url)
        setUpdateVersion(status.latestVersion)
        setUpdateState('available')
      } catch (e) {
        setUpdateError(String(e))
        setUpdateState('error')
      }
      return
    }
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
    if (IS_ANDROID) {
      await handleAutoInstall()
      return
    }
    if (!pendingUpdate) return
    setUpdateState('downloading')
    setDownloadProgress(0)
    setUpdateError(null)
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
    } catch (e) {
      setUpdateError(String(e))
      setUpdateState('error')
      return
    }
    // ponytail: WindowsではMSI導入時にupdaterが自動終了させる。生き残っていたら再起動する
    try {
      await relaunch()
    } catch (e) {
      setUpdateError(String(e))
      setUpdateState('error')
    }
  }

  async function handleAutoInstall() {
    const version = updateVersion()
    if (!version || !pendingApkUrl) return
    setUpdateState('downloading')
    setDownloadProgress(0)
    setUpdateError(null)
    setPermissionNeeded(false)
    try {
      // ponytail: 同版の再試行では再取得しない
      if (!pendingApkPath || pendingApkVersion !== version) {
        const unlisten = await listen<{ progress: number }>('android-update-progress', (event) => {
          if (typeof event.payload?.progress === 'number') {
            setDownloadProgress(event.payload.progress)
          }
        })
        try {
          pendingApkPath = await invoke<string>('download_android_update', { url: pendingApkUrl, version })
          pendingApkVersion = version
        } finally {
          unlisten()
        }
      }
      const allowed = await invoke<boolean>('get_install_permission_status')
      if (!allowed) {
        setPermissionNeeded(true)
        setUpdateState('available')
        return
      }
      setUpdateState('installing')
      await invoke('install_android_update', { path: pendingApkPath })
      // 導入画面を開いた後はOS側の操作になる。成功時はプロセスが置換されるため、このまま待機表示でよい。
    } catch (e) {
      setUpdateError(String(e))
      setUpdateState('error')
    }
  }

  async function handleOpenInstallSettings() {
    try {
      await invoke('open_install_permission_settings')
    } catch (e) {
      setUpdateError(String(e))
    }
  }

  function handleOpenReleasePage() {
    const version = updateVersion()
    if (!version) return
    openUrl(`${repoUrl}/releases/tag/${version}`)
  }

  return (
    <div class="settings-panel">
      <h2 class="page-heading">{t("nav.updates")}</h2>

      <div class="settings-card">
        <h3 class="card-heading">{t("nav.updates")}</h3>

        <div class="update-row">
          <div class="update-meta">
            <span class="update-meta-label">{t("updates.installed_label")}</span>
            <span class="update-version-tag">{appVersion()}</span>
            <p class="switch-desc">{t("updates.current_description")}</p>
          </div>

          <div class="update-action">
            <Show when={updateState() === 'downloading'}>
              <div class="update-progress">
                <p class="update-status">{t("updates.download_progress", { progress: downloadProgress() })}</p>
                <div class="progress-bar">
                  <div class="progress-bar-fill" style={{ width: `${downloadProgress()}%` }} />
                </div>
              </div>
            </Show>

            <Show when={updateState() === 'available'}>
              <Show when={updateError()}>
                <p class="update-error">{updateError()}</p>
              </Show>
              <p class="update-status update-available">
                {t("updates.update_available", { version: updateVersion()! })}
              </p>
              <Show when={IS_ANDROID && permissionNeeded()}>
                <p class="update-status">{t("updates.install_permission_required")}</p>
                <button onClick={handleOpenInstallSettings} class="btn-update">
                  {t("permissions.open_settings")}
                </button>
              </Show>
              <Show when={IS_ANDROID && autoAvailable()}>
                <button onClick={handleInstall} class="btn-install">
                  {t("updates.auto_install")}
                </button>
                <button onClick={handleOpenReleasePage} class="link-button">
                  {t("updates.open_in_browser")}
                </button>
              </Show>
              <Show when={IS_ANDROID && !autoAvailable()}>
                <button onClick={handleOpenReleasePage} class="btn-install">
                  {t("updates.open_in_browser")}
                </button>
              </Show>
              <Show when={!IS_ANDROID}>
                <button onClick={handleInstall} class="btn-install">
                  {t("updates.install_restart")}
                </button>
              </Show>
            </Show>

            <Show when={updateState() === 'installing'}>
              <p class="update-status">{t("updates.opening_installer")}</p>
            </Show>

            <Show when={updateState() === 'uptodate'}>
              <p class="update-status update-ok">{t("updates.up_to_date")}</p>
              <button onClick={handleCheckUpdate} class="btn-update">
                {t("updates.check_again")}
              </button>
            </Show>

            <Show when={updateState() === 'checking'}>
              <p class="update-status">{t("updates.checking")}</p>
              <button class="btn-update" disabled>
                {t("updates.check")}
              </button>
            </Show>

            <Show when={updateState() === 'idle' || updateState() === 'error'}>
              <Show when={updateError()}>
                <p class="update-error">{updateError()}</p>
              </Show>
              <button onClick={handleCheckUpdate} class="btn-update">
                {updateState() === 'error' ? t("updates.check_again") : t("updates.check")}
              </button>
            </Show>
          </div>
        </div>

        <div class="setting-sep" />

        <div class="link-row">
          <button class="link-button" onClick={() => openUrl(repoUrl)}>
            <IconBrandGithub size={14} />
            {t("common.repository")}
          </button>
          <button class="link-button" onClick={() => openUrl(changelogUrl)}>
            <ScrollText size={14} />
            {t("common.changelog")}
          </button>
        </div>
      </div>
    </div>
  )
}