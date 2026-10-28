const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');
const http = require('http');

async function capture() {
  const outDir = path.resolve(__dirname, '..', 'docs', 'screenshots');
  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
  }

  const webDir = path.resolve(__dirname, '..', 'web');
  const server = http.createServer((req, res) => {
    let reqPath = req.url.split('?')[0];
    if (reqPath === '/' || reqPath === '') reqPath = '/index.html';

    if (reqPath === '/api/status') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        status: 'ok',
        version: 1,
        bridge_active: true,
        speech_prob: 0.85,
        capture_device: 'fifine Microphone',
        render_device: 'VB-Audio Virtual Cable',
        spectrum_bins: Array(32).fill(0.1)
      }));
      return;
    }
    if (reqPath === '/api/devices') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        capture: [{ id: 'fifine', name: 'fifine SC3 USB Microphone', default: true }],
        render: [{ id: 'cable', name: 'CABLE Input (VB-Audio Virtual Cable)', default: true }]
      }));
      return;
    }
    if (reqPath === '/api/config') {
      let cfg = '';
      try {
        cfg = fs.readFileSync(path.resolve(__dirname, '..', 'config.txt'), 'utf8');
      } catch (e) {
        cfg = 'Preamp: -2 dB\nFilter 1: ON PK Fc 2500 Hz Gain 4.5 dB Q 1.41\n';
      }
      res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end(cfg);
      return;
    }

    const filePath = path.join(webDir, reqPath);
    if (fs.existsSync(filePath) && fs.statSync(filePath).isFile()) {
      const ext = path.extname(filePath).toLowerCase();
      const mimeTypes = {
        '.html': 'text/html; charset=utf-8',
        '.js': 'application/javascript; charset=utf-8',
        '.css': 'text/css; charset=utf-8',
        '.json': 'application/json; charset=utf-8',
        '.png': 'image/png',
        '.ico': 'image/x-icon'
      };
      res.writeHead(200, { 'Content-Type': mimeTypes[ext] || 'application/octet-stream' });
      fs.createReadStream(filePath).pipe(res);
    } else {
      res.writeHead(404);
      res.end('Not found');
    }
  });

  await new Promise((resolve, reject) => {
    server.listen(49200, '127.0.0.1', () => {
      console.log('Internal static server running on http://127.0.0.1:49200');
      resolve();
    }).on('error', (err) => {
      if (err.code === 'EADDRINUSE') {
        console.log('Port 49200 already in use, proceeding to connect...');
        resolve();
      } else {
        reject(err);
      }
    });
  });

  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });

  const context = await browser.newContext({
    viewport: { width: 1360, height: 860 },
    deviceScaleFactor: 1.5,
  });

  const page = await context.newPage();
  console.log('Navigating to http://127.0.0.1:49200 ...');
  await page.goto('http://127.0.0.1:49200', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(2000);

  // Switch to preset_2 tab if available to show rich filters & RNNoise
  await page.evaluate(() => {
    const tabHeaders = Array.from(document.querySelectorAll('.tab-header'));
    const p2Tab = tabHeaders.find(th => th.textContent.includes('preset_2') || th.textContent.includes('preset'));
    if (p2Tab) p2Tab.click();
  });
  await page.waitForTimeout(1000);

  // 1. Full Main Studio (Dark Theme)
  console.log('Capturing 01_main_studio_dark.png...');
  await page.screenshot({ path: path.join(outDir, '01_main_studio_dark.png') });

  // 2. Interactive FabFilter EQ Node Tooltip Card
  console.log('Pinning EQ canvas node...');
  await page.evaluate(() => {
    if (window.interactiveNodes && window.interactiveNodes.length > 0) {
      const node = window.interactiveNodes[1] || window.interactiveNodes[0];
      window.selectedNode = node;
      if (typeof renderCurveTooltip === 'function') {
        renderCurveTooltip(node);
      }
      if (typeof renderAnalysis === 'function') {
        renderAnalysis();
      }
    }
  });
  await page.waitForTimeout(500);

  const eqArea = await page.$('.analysis-panel, #curveCanvas');
  if (eqArea) {
    console.log('Capturing 02_fabfilter_eq_node.png (cropped)...');
    await eqArea.screenshot({ path: path.join(outDir, '02_fabfilter_eq_node.png') });
  } else {
    await page.screenshot({ path: path.join(outDir, '02_fabfilter_eq_node.png') });
  }

  // 3. RNNoise AI card
  console.log('Capturing 03_neural_ai_rnnoise.png...');
  const rnnoiseBlock = await page.$('.filter-block-card:has(.rnnoise-desc)');
  if (rnnoiseBlock) {
    await rnnoiseBlock.screenshot({ path: path.join(outDir, '03_neural_ai_rnnoise.png') });
  } else {
    // Fallback: take first filter block
    const firstBlock = await page.$('.filter-block-card');
    if (firstBlock) await firstBlock.screenshot({ path: path.join(outDir, '03_neural_ai_rnnoise.png') });
  }

  // 4. VB-Cable Modal
  console.log('Opening VB-Cable modal...');
  await page.evaluate(() => {
    const modal = document.getElementById('vbCableModal');
    if (modal) modal.classList.remove('hidden');
  });
  await page.waitForTimeout(500);
  const modalDialog = await page.$('#vbCableModal .win-dialog');
  if (modalDialog) {
    await modalDialog.screenshot({ path: path.join(outDir, '04_vbcable_modal.png') });
  } else {
    await page.screenshot({ path: path.join(outDir, '04_vbcable_modal.png') });
  }

  // Close modal
  await page.evaluate(() => {
    const modal = document.getElementById('vbCableModal');
    if (modal) modal.classList.add('hidden');
    // Deselect node
    window.selectedNode = null;
    const tt = document.getElementById('curveTooltip');
    if (tt) tt.classList.add('hidden');
    if (typeof renderAnalysis === 'function') renderAnalysis();
  });

  // 5. Light Theme
  console.log('Capturing 05_main_studio_light.png...');
  await page.evaluate(() => {
    document.body.classList.remove('theme-dark');
  });
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(outDir, '05_main_studio_light.png') });

  // 6. Clean Add Effect Menu (back to dark theme)
  console.log('Capturing 06_add_effect_menu.png...');
  await page.evaluate(() => {
    document.body.classList.add('theme-dark');
    const btn = document.getElementById('addBlockBtn');
    if (btn) btn.click();
    const cat = document.querySelector('.menu-category[data-cat="rc505"]');
    if (cat) cat.classList.add('hover-active');
  });
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(outDir, '06_add_effect_menu.png') });

  await browser.close();
  server.close();
  console.log('All screenshots captured successfully in:', outDir);
}

capture().catch(err => {
  console.error('Capture error:', err);
  process.exit(1);
});
