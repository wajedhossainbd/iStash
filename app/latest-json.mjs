// Prints the file the in-app updater reads (latest.json) for one release.
// Usage: node app/latest-json.mjs <version> <path to iStash.app.tar.gz.sig> <owner/repo>
import { readFileSync } from 'node:fs';

const [version, sig, repo] = process.argv.slice(2);
if (!version || !sig || !repo) throw new Error('usage: latest-json.mjs <version> <sig file> <owner/repo>');
console.log(JSON.stringify({
  version,
  pub_date: new Date().toISOString(),
  notes: 'See the release page.',
  platforms: {
    'darwin-aarch64': {
      signature: readFileSync(sig, 'utf8').trim(),
      url: `https://github.com/${repo}/releases/download/v${version}/iStash.app.tar.gz`,
    },
  },
}, null, 2));
