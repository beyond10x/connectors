// A static server for the built site, the way GitHub Pages serves it under /connectors/ with
// trailingSlash false: /connectors/docs/x is build/docs/x.html or build/docs/x/index.html. It starts
// in milliseconds, where `docusaurus serve` loads the whole site config first.
// Usage: node tests/ux/serve.mjs <port>
import {createReadStream, existsSync, statSync} from 'node:fs';
import {createServer} from 'node:http';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..', 'build');
const base = '/connectors';
const types = {'.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.woff2': 'font/woff2', '.woff': 'font/woff', '.wasm': 'application/wasm', '.xml': 'application/xml', '.txt': 'text/plain', '.ico': 'image/x-icon'};

function resolve(pathname) {
  if (!pathname.startsWith(base)) return null;
  const rel = decodeURIComponent(pathname.slice(base.length)).replace(/\/+$/, '');
  const at = path.join(root, rel);
  if (!at.startsWith(root)) return null;
  for (const candidate of [at, at + '.html', path.join(at, 'index.html')]) if (existsSync(candidate) && statSync(candidate).isFile()) return candidate;
  return null;
}

createServer((req, res) => {
  const url = new URL(req.url ?? '/', 'http://local');
  const file = resolve(url.pathname);
  const status = file ? 200 : 404;
  const send = file ?? path.join(root, '404.html');
  res.writeHead(status, {'content-type': types[path.extname(send)] ?? 'application/octet-stream', 'cache-control': 'no-store'});
  createReadStream(send).pipe(res);
}).listen(Number(process.argv[2] ?? 3141), '127.0.0.1');
