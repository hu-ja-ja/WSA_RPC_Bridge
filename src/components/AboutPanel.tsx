import { createResource } from 'solid-js'
import { openUrl } from '@tauri-apps/plugin-opener'
import { getVersion } from '@tauri-apps/api/app'
import { IconBrandGithub } from '@tabler/icons-solidjs'
import { BadgeInfo, ScrollText } from 'lucide-solid'
import { t, locale } from '../i18n'

const APP_NAME = 'WSA RPC Bridge'
const [appVersion] = createResource(async () => `v${await getVersion()}`)
const COPYRIGHT = 'Copyright (C) 2026 hu-ja-ja'

const privacyPolicyUrl =
  locale === 'ja'
    ? 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/PRIVACY_POLICY.md'
    : 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/PRIVACY_POLICY_en.md'

const repoUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge'
const changelogUrl = 'https://github.com/hu-ja-ja/WSA_RPC_Bridge/blob/main/CHANGELOG.md'

export function AboutPanel() {
  return (
    <div class="about-panel">
      <div class="about-hero">
        <BadgeInfo class="about-icon" size={28} />
        <div class="about-hero-body">
          <p class="about-app-name">{APP_NAME}</p>
          <p class="about-version">{appVersion()}</p>
        </div>
      </div>

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
        </div>
      </div>
    </div>
  )
}
