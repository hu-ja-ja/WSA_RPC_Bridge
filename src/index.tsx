/* @refresh reload */
import { render } from 'solid-js/web'
import './index.css'
import App from './App.tsx'
import { locale } from './i18n'

document.documentElement.lang = locale

const root = document.getElementById('root')

if (!root) {
  throw new Error('Root element #root not found')
}

render(() => <App />, root)
