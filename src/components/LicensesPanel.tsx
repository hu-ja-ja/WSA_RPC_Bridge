import { t } from '../i18n'
import { licenses } from '../generated/licenses'
import type { LicenseEntry } from '../generated/licenses'

const entries: LicenseEntry[] = licenses

export function LicensesPanel() {
  return (
    <div class="licenses-panel">
      <div class="license-about-card">
        <p class="license-app-name">WSA RPC Bridge v0.1.0</p>
        <p class="license-copyright">Copyright (C) 2026 hu-ja-ja</p>
        <p class="license-project-license">{t('licenses.mpl_notice')}</p>
      </div>

      <div class="settings-divider" />

      <h3 class="section-title">{t('licenses.third_party_title')}</h3>

      <div class="license-list">
        {entries.map((e) => (
          <details class="license-item">
            <summary class="license-header">
              <span class="license-name">{e.name}</span>
              <span class="license-badge">{e.license}</span>
            </summary>
            <div class="license-body">
              <p class="license-meta">
                {t('licenses.version')}: {e.version}<br />
                {e.copyright}<br />
                <a href={e.url} target="_blank" rel="noopener noreferrer">{e.url}</a>
              </p>
              <pre class="license-text">{e.text}</pre>
            </div>
          </details>
        ))}
      </div>
    </div>
  )
}
