"""Run the real packaging wrapper with tiny compiler outputs, then reuse them."""
import functools, http.server, json, os, shutil, subprocess, tempfile, threading, unittest
from pathlib import Path
from web_engine_inputs import fingerprint

class WrapperTests(unittest.TestCase):
    def test_runtime_only_update_skips_all_compiler_tools(self):
        repo=Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/'repo'; root.mkdir()
            for name in ['scripts/build_web.sh','scripts/stamp_web_build_id.py','tools/web_engine_inputs.py','tools/reuse_web_engine.py','tools/generate_web_manifest.py','tools/package_web.py','tools/retain_web_releases.py','web/sw.js']:
                dst=root/name;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/name,dst)
            (root/'src').mkdir();(root/'src/main.rs').write_text('fn main() {}')
            (root/'Cargo.toml').write_text('[package]\nname="fixture"\nversion="0.1.0"\n')
            (root/'Cargo.lock').write_text('[[package]]\nname = "wasm-bindgen"\nversion = "0.2.128"\n')
            (root/'web/index.html').write_text('<script>const buildId = "fixture";</script>')
            (root/'assets/data').mkdir(parents=True);(root/'assets/data/balance.json').write_text('{"hp":1}')
            bindir=root/'bin';bindir.mkdir()
            commands={
                'cargo':'echo cargo >> "$CALLS"; mkdir -p target/wasm32-unknown-unknown/release; printf wasm > target/wasm32-unknown-unknown/release/bevy-open-rts.wasm',
                'wasm-bindgen':'if [ "$1" = --version ]; then echo "wasm-bindgen 0.2.128"; exit; fi; echo bindgen >> "$CALLS"; printf engine > web/pkg/bevy_open_rts_bg.wasm; printf glue > web/pkg/bevy_open_rts.js',
                'wasm-opt':'if [ "$1" = --version ]; then echo "wasm-opt version 130"; exit; fi; last=""; output=""; next=0; for a in "$@"; do if [ "$next" = 1 ]; then output="$a"; next=0; fi; if [ "$a" = -o ]; then next=1; fi; last="$a"; done; cp "$last" "$output"',
                'git':'echo fixture-revision',
            }
            for name,script in commands.items():
                p=bindir/name;p.write_text('#!/usr/bin/env bash\nset -e\n'+script+'\n');p.chmod(0o755)
            env=dict(os.environ,PATH=str(bindir)+os.pathsep+os.environ['PATH'],CARGO_HOME=str(root/'empty-cargo'),CALLS=str(root/'calls'))
            env.pop('WASM_REUSE_BASE_URL',None)
            def build():
                result=subprocess.run(['bash','scripts/build_web.sh'],cwd=root,env=env,capture_output=True,text=True,timeout=60)
                self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            build();self.assertIn('cargo',(root/'calls').read_text())
            old=json.loads((root/'target/web-dist/build.json').read_text());self.assertEqual(old['engine_inputs_sha256'],fingerprint(root)[0])
            published=Path(temp)/'published';shutil.copytree(root/'target/web-dist',published)
            server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(http.server.SimpleHTTPRequestHandler,directory=str(published)))
            thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
            try:
                env['WASM_REUSE_BASE_URL']=f'http://127.0.0.1:{server.server_port}'
                (root/'calls').write_text('');(root/'assets/data/balance.json').write_text('{"hp":2}')
                (root/'web/index.html').write_text('<h1>Updated</h1><script>const buildId = "fixture";</script>')
                build();self.assertEqual((root/'calls').read_text(),'')
                new=json.loads((root/'target/web-dist/build.json').read_text());self.assertEqual(old['engine_inputs_sha256'],new['engine_inputs_sha256']);self.assertEqual(old['wasm_sha256'],new['wasm_sha256'])
                self.assertEqual((root/'target/web-dist/assets/data/balance.json').read_text(),'{"hp":2}')
            finally:server.shutdown();server.server_close();thread.join()
