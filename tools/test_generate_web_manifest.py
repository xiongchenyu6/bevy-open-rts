import hashlib
import tempfile
import unittest
from pathlib import Path
from generate_web_manifest import generate, TOKEN


class ManifestTests(unittest.TestCase):
    def test_reproducible_content_identity_and_download_variants(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            root = base / 'dist'
            root.mkdir()
            template = base / 'worker.js'
            template.write_text('const MANIFEST = ' + TOKEN + ';')
            inputs = {'index.html': b'<html>game</html>', 'game.js': b'game',
                      'game.wasm': b'raw', 'game.wasm.gz': b'gzip',
                      'assets/model.glb': b'model', 'build.json': b'receipt'}
            for name, data in inputs.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            first = generate(root, 'revision', template)
            second = generate(root, 'revision', template)
            self.assertEqual(first, second)
            entries = {f['path']: f for f in first['files']}
            self.assertEqual(set(entries), set(inputs) - {'build.json'})
            self.assertEqual(entries['game.wasm']['encoding'], 'raw')
            self.assertEqual(entries['game.wasm.gz']['encoding'], 'gzip')
            self.assertEqual(entries['assets/model.glb']['sha256'], hashlib.sha256(b'model').hexdigest())
            self.assertEqual(entries['assets/model.glb']['bytes'], 5)
            self.assertNotIn(TOKEN, (root / 'sw.js').read_text())
            (root / 'build.json').write_text('new receipt')
            self.assertEqual(first, generate(root, 'revision', template))
            (root / 'game.js').write_text('changed')
            self.assertNotEqual(first['version'], generate(root, 'revision', template)['version'])
            changed = generate(root, 'revision', template)
            metadata_only = generate(root, 'other-revision', template)
            self.assertEqual(changed['version'], metadata_only['version'])
            self.assertNotEqual(changed['revision'], metadata_only['revision'])

    def test_missing_entrypoint_and_invalid_template_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            template = root / 'sw.js'
            template.write_text(TOKEN)
            with self.assertRaisesRegex(ValueError, 'index.html'):
                generate(root, 'test', template)
            (root / 'index.html').write_text('game')
            template.write_text('no placeholder')
            with self.assertRaisesRegex(ValueError, 'placeholder'):
                generate(root, 'test', template)


if __name__ == '__main__':
    unittest.main()
