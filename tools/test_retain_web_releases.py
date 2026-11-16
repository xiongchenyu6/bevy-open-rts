import hashlib
import http.server
import json
from pathlib import Path
import tempfile
import threading
import unittest
from functools import partial
from generate_web_manifest import generate, TOKEN
from retain_web_releases import retain


class RetentionTests(unittest.TestCase):
    def test_retained_resources_survive_releases_and_corruption_blocks_publish(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            published = base / 'published'
            published.mkdir()
            template = base / 'template.js'
            template.write_text(TOKEN)
            (published / 'index.html').write_text('old page')
            (published / 'old.glb').write_bytes(b'old model')
            old = generate(published, 'old', template)
            (published / 'release-index.json').write_text(json.dumps({'releases': [old['version']]}))
            handler = partial(http.server.SimpleHTTPRequestHandler, directory=str(published))
            server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                current = base / 'current'
                current.mkdir()
                (current / 'index.html').write_text('new page')
                new = generate(current, 'new', template)
                url = f'http://127.0.0.1:{server.server_port}'
                retain(current, url)
                history = json.loads((current / 'release-index.json').read_text())['releases']
                self.assertEqual(history, [new['version'], old['version']])
                for file in old['files']:
                    self.assertEqual(hashlib.sha256((current / file['url']).read_bytes()).hexdigest(), file['sha256'])
                # A fresh build must refuse historical bytes that do not match the manifest.
                model = next(f for f in old['files'] if f['path'] == 'old.glb')
                (published / model['url']).write_bytes(b'bad model')
                (current / model['url']).unlink()
                with self.assertRaisesRegex(ValueError, 'integrity'):
                    retain(current, url)
            finally:
                server.shutdown()
                server.server_close()
                thread.join()

    def test_history_keeps_exactly_two_previous_releases(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            published = base / 'published'
            published.mkdir()
            template = base / 'template.js'
            template.write_text(TOKEN)
            history = []
            for index in range(3):
                (published / 'index.html').write_text(f'old page {index}')
                history.insert(0, generate(published, str(index), template))
            (published / 'release-index.json').write_text(json.dumps({'releases': [m['version'] for m in history]}))
            server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), partial(http.server.SimpleHTTPRequestHandler, directory=str(published)))
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                current = base / 'current'
                current.mkdir()
                (current / 'index.html').write_text('latest')
                latest = generate(current, 'latest', template)
                retain(current, f'http://127.0.0.1:{server.server_port}')
                self.assertEqual(json.loads((current / 'release-index.json').read_text())['releases'], [latest['version'], history[0]['version'], history[1]['version']])
                for manifest in history[:2]:
                    self.assertTrue((current / manifest['files'][0]['url']).exists())
                self.assertFalse((current / history[2]['files'][0]['url']).exists())
            finally:
                server.shutdown()
                server.server_close()
                thread.join()

    def test_manifest_groups_and_immutable_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            template = root / 'sw.js'
            template.write_text(TOKEN)
            files = {'index.html': 'core', 'assets/data/campaign.json': 'core',
                     'assets/ui/icons/fire.png': 'core', 'assets/models/boss.glb': 'combat',
                     'assets/audio/shot.wav': 'combat', 'THIRD_PARTY_NOTICES/license.txt': 'optional'}
            for path in files:
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(path.encode())
            manifest = generate(root, 'test', template)
            for file in manifest['files']:
                self.assertEqual(file['group'], files[file['path']])
                self.assertEqual((root / file['url']).read_bytes(), (root / file['path']).read_bytes())
