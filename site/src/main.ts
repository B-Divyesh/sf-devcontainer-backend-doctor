type Severity = 'error' | 'warning' | 'info'

interface DemoFinding {
  severity: Severity
  id: string
  title: string
  detail: string
}

interface DemoReport {
  verdict: string
  verdictClass: string
  counts: string
  findings: DemoFinding[]
}

const reports: Record<string, DemoReport> = {
  'docker-desktop': {
    verdict: 'REVIEW', verdictClass: 'status-warning', counts: '0 errors · 3 warnings',
    findings: [
      { severity: 'warning', id: 'NET-HOST-MODE', title: 'Host network needs Docker Desktop 4.34+', detail: 'Enable the setting or publish explicit ports.' },
      { severity: 'warning', id: 'ARCH-MISMATCH', title: 'amd64 differs from this arm64 host', detail: 'Emulation works, but a multi-architecture image is safer.' },
      { severity: 'info', id: 'MOUNT-DOCKER-SOCKET', title: 'Docker socket grants host daemon control', detail: 'Prefer a scoped socket proxy where possible.' }
    ]
  },
  podman: {
    verdict: 'REVIEW', verdictClass: 'status-warning', counts: '0 errors · 4 warnings',
    findings: [
      { severity: 'warning', id: 'MOUNT-DOCKER-SOCKET', title: 'Docker socket path is not portable', detail: 'Mount the enabled Podman API socket instead.' },
      { severity: 'warning', id: 'NET-HOST-MODE', title: 'Host mode targets the Podman machine', detail: 'Publish ports and use host.containers.internal.' },
      { severity: 'warning', id: 'ARCH-MISMATCH', title: 'amd64 emulation requires binfmt/QEMU', detail: 'Publish a multi-architecture image.' }
    ]
  },
  orbstack: {
    verdict: 'REVIEW', verdictClass: 'status-warning', counts: '0 errors · 2 warnings',
    findings: [
      { severity: 'warning', id: 'NET-HOST-MODE', title: 'Host mode remains VM-scoped', detail: 'Publish explicit ports and verify reachability.' },
      { severity: 'warning', id: 'ARCH-MISMATCH', title: 'amd64 will run through emulation', detail: 'Prefer an image matching the Mac host.' },
      { severity: 'info', id: 'MOUNT-DOCKER-SOCKET', title: 'Docker-compatible socket is available', detail: 'The mount still grants daemon control.' }
    ]
  },
  'apple-container': {
    verdict: 'BLOCKED', verdictClass: 'status-error', counts: '4 errors · 1 warning',
    findings: [
      { severity: 'error', id: 'APPLE-COMPOSE-UNSUPPORTED', title: 'Compose project cannot run directly', detail: 'Keep a Docker-compatible backend or replace orchestration.' },
      { severity: 'error', id: 'MOUNT-DOCKER-SOCKET', title: 'No Docker-compatible socket', detail: 'Remove the daemon dependency.' },
      { severity: 'error', id: 'NET-HOST-MODE', title: 'Docker-style host networking is unavailable', detail: 'Use explicit published ports.' }
    ]
  }
}

const tabs = Array.from(document.querySelectorAll<HTMLButtonElement>('[role="tab"]'))
const panel = document.querySelector<HTMLElement>('#diagnostic-panel')
const findings = document.querySelector<HTMLOListElement>('#demo-findings')
const command = document.querySelector<HTMLElement>('#demo-command')
const backendLabel = document.querySelector<HTMLElement>('#demo-backend')
const verdict = document.querySelector<HTMLElement>('#demo-verdict')
const count = document.querySelector<HTMLElement>('#demo-count')

function renderDemo(backend: string, activeTab: HTMLButtonElement): void {
  const report = reports[backend]
  if (!report || !panel || !findings || !command || !backendLabel || !verdict || !count) return
  tabs.forEach((tab) => {
    const selected = tab === activeTab
    tab.setAttribute('aria-selected', String(selected))
    tab.tabIndex = selected ? 0 : -1
  })
  panel.setAttribute('aria-labelledby', activeTab.id)
  command.textContent = `devcontainer-backend-doctor check . --backend ${backend}`
  backendLabel.textContent = backend
  verdict.textContent = report.verdict
  verdict.className = report.verdictClass
  count.textContent = report.counts
  findings.replaceChildren(...report.findings.map((item) => {
    const row = document.createElement('li')
    row.className = `finding finding-${item.severity}`
    const marker = document.createElement('span')
    marker.className = 'finding-marker'
    marker.textContent = item.severity === 'error' ? '×' : item.severity === 'warning' ? '!' : 'i'
    marker.setAttribute('aria-label', item.severity)
    const body = document.createElement('div')
    const heading = document.createElement('strong')
    heading.textContent = item.title
    const rule = document.createElement('code')
    rule.textContent = item.id
    const detail = document.createElement('p')
    detail.textContent = item.detail
    body.append(heading, rule, detail)
    row.append(marker, body)
    return row
  }))
}

tabs.forEach((tab, index) => {
  tab.addEventListener('click', () => renderDemo(tab.dataset.backend ?? '', tab))
  tab.addEventListener('keydown', (event) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
    event.preventDefault()
    let next = index
    if (event.key === 'ArrowLeft') next = (index - 1 + tabs.length) % tabs.length
    if (event.key === 'ArrowRight') next = (index + 1) % tabs.length
    if (event.key === 'Home') next = 0
    if (event.key === 'End') next = tabs.length - 1
    const target = tabs[next]
    target.focus()
    renderDemo(target.dataset.backend ?? '', target)
  })
})

if (tabs[0]) renderDemo('docker-desktop', tabs[0])

const copyButton = document.querySelector<HTMLButtonElement>('#copy-command')
const installCode = document.querySelector<HTMLElement>('#install-code')
const copyStatus = document.querySelector<HTMLElement>('#copy-status')
copyButton?.addEventListener('click', async () => {
  const value = installCode?.textContent ?? ''
  try {
    await navigator.clipboard.writeText(value)
    if (copyStatus) copyStatus.textContent = 'Install command copied.'
    copyButton.textContent = 'Copied'
  } catch {
    if (copyStatus) copyStatus.textContent = 'Copy was blocked. Select the command and copy it manually.'
  }
})

if ('serviceWorker' in navigator && location.protocol === 'https:') {
  window.addEventListener('load', () => navigator.serviceWorker.register('/sw.js').catch(() => undefined))
}
