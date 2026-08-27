import AxeBuilder from '@axe-core/playwright'
import { expect, test } from '@playwright/test'

test('landing page is operable, responsive, and free of serious accessibility issues', async ({ page }, testInfo) => {
  const consoleErrors: string[] = []
  page.on('console', (message) => {
    if (message.type() === 'error') consoleErrors.push(message.text())
  })
  page.on('pageerror', (error) => consoleErrors.push(error.message))

  await page.goto('/')
  await expect(page).toHaveTitle(/Devcontainer Backend Doctor/)
  await expect(page.locator('main')).toHaveCount(1)
  await expect(page.locator('h1')).toHaveCount(1)
  await expect(page.locator('img[alt]')).toHaveCount(1)
  await expect(page.locator('body')).toBeVisible()

  const documentWidth = await page.evaluate(() => document.documentElement.scrollWidth)
  const viewportWidth = page.viewportSize()?.width ?? 0
  expect(documentWidth).toBeLessThanOrEqual(viewportWidth)

  const podmanTab = page.getByRole('tab', { name: 'Podman' })
  await podmanTab.click()
  await expect(podmanTab).toHaveAttribute('aria-selected', 'true')
  await expect(page.locator('#demo-backend')).toHaveText('podman')
  await podmanTab.press('ArrowRight')
  await expect(page.getByRole('tab', { name: 'OrbStack' })).toHaveAttribute('aria-selected', 'true')

  const accessibility = await new AxeBuilder({ page }).analyze()
  const serious = accessibility.violations.filter((violation) => ['serious', 'critical'].includes(violation.impact ?? ''))
  expect(serious).toEqual([])
  expect(consoleErrors).toEqual([])

  await page.screenshot({ path: testInfo.outputPath('landing.png'), fullPage: true })
})

test('legal routes render with one main heading', async ({ page }) => {
  for (const route of ['/privacy/', '/terms/']) {
    await page.goto(route)
    await expect(page.locator('main')).toHaveCount(1)
    await expect(page.locator('h1')).toHaveCount(1)
  }
})
