import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.join(__dirname, '..');
const PUBLIC_FONTS_DIR = path.join(PROJECT_ROOT, 'public/fonts');
const FONTS_CSS_URL = 'https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&family=Space+Grotesk:wght@400;500;600;700&display=swap&format=woff2';

async function download(url: string, dest: string): Promise<void> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch ${url}: ${res.status}`);
  const data = await res.arrayBuffer();
  try {
    fs.writeFileSync(dest, Buffer.from(data));
  } catch (err) {
    throw new Error(`Failed to write ${dest}: ${err}`);
  }
}

async function main(): Promise<void> {
  if (!fs.existsSync(PUBLIC_FONTS_DIR)) {
    fs.mkdirSync(PUBLIC_FONTS_DIR, { recursive: true });
  }

  console.log('Fetching fonts CSS...');
  const cssRes = await fetch(FONTS_CSS_URL);
  if (!cssRes.ok) {
    throw new Error(`Failed to fetch ${FONTS_CSS_URL}: ${cssRes.status}`);
  }
  const cssText = await cssRes.text();

  const urlRegex = /url\((https:\/\/fonts\.gstatic\.com\/[^)]+\.(?:ttf|woff2))\)/g;
  const urls = new Set<string>();
  let match;
  while ((match = urlRegex.exec(cssText)) !== null) {
    urls.add(match[1]);
  }

  console.log(`Found ${urls.size} font files to download...`);

  for (const url of urls) {
    const filename = path.basename(url);
    const dest = path.join(PUBLIC_FONTS_DIR, filename);
    if (fs.existsSync(dest)) {
      console.log(`  Already exists: ${filename}`);
      continue;
    }
    console.log(`  Downloading: ${filename}`);
    await download(url, dest);
  }

  const localCss = cssText.replace(
    /url\(https:\/\/fonts\.gstatic\.com\/(?:[^/]+\/)+([^)]+)\)/g,
    'url(/fonts/$1)'
  );

  fs.writeFileSync(path.join(PUBLIC_FONTS_DIR, 'fonts.css'), localCss);
  console.log('Generated local fonts.css');
  console.log('Done!');
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
