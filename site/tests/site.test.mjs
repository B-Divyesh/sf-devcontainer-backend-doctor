import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const source = await readFile(new URL('../index.html', import.meta.url), 'utf8')
const styles = await readFile(new URL('../src/style.css', import.meta.url), 'utf8')
const script = await readFile(new URL('../src/main.ts', import.meta.url), 'utf8')

test('landing page has the required semantic shell', () => {
  assert.match(source, /<html lang="en">/)
  assert.equal((source.match(/<h1[ >]/g) ?? []).length, 1)
  assert.equal((source.match(/<main[ >]/g) ?? []).length, 1)
  assert.match(source, /<title>[^<]+<\/title>/)
  assert.match(source, /<img[^>]+alt="[^"]+"/)
  assert.match(source, /class="skip-link"/)
})

test('motion and keyboard interaction have deliberate fallbacks', () => {
  assert.match(styles, /prefers-reduced-motion:\s*reduce/)
  assert.match(styles, /:focus-visible/)
  assert.match(script, /ArrowLeft/)
  assert.match(script, /ArrowRight/)
})

test('site has no remote scripts, fonts, or analytics', () => {
  assert.doesNotMatch(source, /<script[^>]+https?:\/\//)
  assert.doesNotMatch(source, /fonts\.(googleapis|gstatic)/)
  assert.doesNotMatch(source + script, /(google-analytics|segment\.com|plausible\.io)/i)
})
