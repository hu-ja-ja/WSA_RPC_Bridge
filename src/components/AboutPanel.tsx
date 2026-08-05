import { openUrl } from '@tauri-apps/plugin-opener'
import { BadgeInfo } from 'lucide-solid'
import { t, locale } from '../i18n'

const APP_NAME = 'WSA RPC Bridge'
const APP_VERSION = 'v0.3.1'
const COPYRIGHT = 'Copyright (C) 2026 hu-ja-ja'

const privacyPolicyUrl =
  locale === 'ja'
    ? 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/PRIVACY_POLICY.md'
    : 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/PRIVACY_POLICY_en.md'

const repoUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge'

export function AboutPanel() {
  return (
    <div class="about-panel">
      <div class="about-hero">
        <BadgeInfo class="about-icon" size={28} />
        <div class="about-hero-body">
          <p class="about-app-name">{APP_NAME}</p>
          <p class="about-version">{APP_VERSION}</p>
        </div>
      </div>

      <div class="license-about-card">
        <p class="license-copyright">{COPYRIGHT}</p>
        <p class="license-project-license">{t('licenses.mpl_notice')}</p>
        <a
          class="license-privacy-link"
          href="#"
          onClick={(e) => {
            e.preventDefault()
            openUrl(repoUrl)
          }}
        >
          {t('licenses.repository')}
        </a>
        <a
          class="license-privacy-link"
          href="#"
          onClick={(e) => {
            e.preventDefault()
            openUrl(privacyPolicyUrl)
          }}
        >
          {t('licenses.privacy_policy')}
        </a>
      </div>
    </div>
  )
}
