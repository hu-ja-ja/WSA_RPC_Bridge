import { createResource, createSignal, Show } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { getVersion } from '@tauri-apps/api/app'
import { IconBrandGithub } from '@tabler/icons-solidjs'
import { BadgeInfo, Check, Copy, ScrollText, Handshake } from 'lucide-solid'
import { t } from '../i18n'

const APP_NAME = 'WSA RPC Bridge'
const [appVersion] = createResource(async () => `v${await getVersion()}`)
const [fingerprint] = createResource(async () => await invoke<string | null>('get_signing_fingerprint'))
const COPYRIGHT = 'Copyright (C) 2026 hu-ja-ja'

// ponytail: 法務ページは日本語版のみ公開。英語版復活時に locale 分岐を戻す
const privacyPolicyUrl = 'https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/legal/privacy'

const termsOfServiceUrl = 'https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/legal/terms'

const repoUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge'
const changelogUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/CHANGELOG.md'

export function AboutPanel() {
  const [copied, setCopied] = createSignal(false)

  const copyFingerprint = async () => {
    const fp = fingerprint()
    if (!fp) return
    await navigator.clipboard.writeText(fp)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <div class="about-panel">
      <div class="about-hero">
        <BadgeInfo class="about-icon" size={28} />
        <div class="about-hero-body">
          <p class="about-app-name">{APP_NAME}</p>
          <p class="about-version">{appVersion()}</p>
        </div>
      </div>

      <Show when={fingerprint()}>
        <div class="fingerprint-card">
          <div class="fingerprint-head">
            <p class="fingerprint-title">{t('about.fingerprint')}</p>
            <button class="fingerprint-copy" onClick={copyFingerprint}>
              <Show when={copied()} fallback={<Copy size={13} />}>
                <Check size={13} />
              </Show>
              {copied() ? t('about.copied') : t('about.copy')}
            </button>
          </div>
          <p class="about-fingerprint">{fingerprint()}</p>
          <p class="fingerprint-hint">{t('about.fingerprint_description')}</p>
        </div>
      </Show>

      <div class="license-about-card">
        <p class="license-copyright">{COPYRIGHT}</p>
        <p class="license-project-license">{t('about.license')}</p>
        <div class="setting-sep" />
        <div class="license-links">
          <a
            class="link-button"
            href="#"
            onClick={(e) => {
              e.preventDefault()
              openUrl(repoUrl)
            }}
          >
            <IconBrandGithub size={14} />
            {t('common.repository')}
          </a>
          <a
            class="link-button"
            href="#"
            onClick={(e) => {
              e.preventDefault()
              openUrl(changelogUrl)
            }}
          >
            <ScrollText size={14} />
            {t('common.changelog')}
          </a>
          <a
            class="link-button"
            href="#"
            onClick={(e) => {
              e.preventDefault()
              openUrl(privacyPolicyUrl)
            }}
          >
            <BadgeInfo size={14} />
            {t('common.privacy_policy')}
          </a>
          <a
            class="link-button"
            href="#"
            onClick={(e) => {
              e.preventDefault()
              openUrl(termsOfServiceUrl)
            }}
          >
            <Handshake size={14} />
            {t('common.terms_of_service')}
          </a>
        </div>
      </div>
    </div>
  )
}
