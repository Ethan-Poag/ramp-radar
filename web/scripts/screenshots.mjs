// Usage: node web/scripts/screenshots.mjs [baseUrl] [outDir]   (needs Chrome/Chromium; CHROME_PATH to override)
import puppeteer from 'puppeteer-core';

const base = process.argv[2] ?? 'http://127.0.0.1:8080';
const out = process.argv[3] ?? 'docs/screenshots';
const chrome = process.env.CHROME_PATH ?? '/usr/bin/google-chrome';

const browser = await puppeteer.launch({
  executablePath: chrome,
  headless: true,
  args: ['--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--enable-webgl'],
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const page = await browser.newPage();
page.on('pageerror', (e) => console.log('pageerror:', e.message));

async function shot(name, w, h, scale = 1) {
  await page.setViewport({ width: w, height: h, deviceScaleFactor: scale });
  await page.screenshot({ path: `${out}/${name}.png` });
  console.log('saved', name);
}
const clickTab = (label) =>
  page.evaluate((l) => [...document.querySelectorAll('nav button')].find((b) => b.textContent.trim().endsWith(l) && b.offsetParent)?.click(), label);

for (const theme of ['light', 'dark']) {
  await page.setViewport({ width: 1360, height: 860 });
  await page.goto(base, { waitUntil: 'networkidle2' });
  await page.evaluate((t) => (document.documentElement.dataset.theme = t), theme);
  await sleep(1500);
  await page.evaluate(() => document.querySelector('.tcard')?.click());
  await sleep(1200);
  await shot(theme === 'light' ? 'trackers' : 'trackers-dark', 1360, 860);

  await clickTab('Map');
  await sleep(9000); // tiles + traffic
  await shot(theme === 'light' ? 'map' : 'map-dark', 1360, 860);
  await clickTab('Log');
  await sleep(800);
  if (theme === 'light') await shot('log', 1360, 700);
  if (theme === 'light') {
    await clickTab('Settings');
    await sleep(800);
    await shot('settings', 1360, 1100);
  }
}

// phone
await page.setViewport({ width: 390, height: 844, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
await page.goto(base, { waitUntil: 'networkidle2' });
await page.evaluate(() => (document.documentElement.dataset.theme = 'light'));
await sleep(1500);
await shot('phone-trackers', 390, 844, 2);
await page.evaluate(() => document.querySelector('.tcard')?.click());
await sleep(800);
await shot('phone-editor', 390, 844, 2);
await clickTab('Map');
await sleep(1000);
await page.evaluate(() => [...document.querySelectorAll('.bottombar button')].find((b) => b.textContent.includes('Map'))?.click());
await sleep(8000);
await shot('phone-map', 390, 844, 2);
await browser.close();
