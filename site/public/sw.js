const CACHE = 'backend-doctor-v2'
const SHELL = ['/', '/privacy/', '/terms/', '/harbor-crossing.webp', '/favicon.svg']

self.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE).then(async (cache) => {
    await cache.addAll(SHELL)
    const home = await fetch('/')
    const html = await home.text()
    const builtAssets = [...html.matchAll(/(?:src|href)="(\/assets\/[^\"]+)"/g)].map((match) => match[1])
    await cache.addAll(builtAssets)
  }))
  self.skipWaiting()
})

self.addEventListener('activate', (event) => {
  event.waitUntil(caches.keys().then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key)))))
  self.clients.claim()
})

self.addEventListener('fetch', (event) => {
  if (event.request.method !== 'GET') return
  event.respondWith(fetch(event.request).then((response) => {
    const copy = response.clone()
    caches.open(CACHE).then((cache) => cache.put(event.request, copy))
    return response
  }).catch(() => caches.match(event.request).then((cached) => cached || caches.match('/'))))
})
