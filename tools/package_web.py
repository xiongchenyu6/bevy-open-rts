#!/usr/bin/env python3
"""Stage only runtime files; never publish local QA captures or logs."""
import hashlib, json, shutil, subprocess, sys
from pathlib import Path
from web_engine_inputs import fingerprint
from generate_web_manifest import generate
current_inputs=fingerprint(Path.cwd())[0]
if len(sys.argv)>1 and sys.argv[1]!=current_inputs:
    raise SystemExit('Compiled inputs changed during build; rebuild before publishing')
root=Path('target/web-dist')
shutil.rmtree(root, ignore_errors=True)
root.mkdir(parents=True)
for name in ('index.html','styles.css','manifest.webmanifest','game-cache.js','game-cache.css','downloads.js','pwa.js','pwa.css','touch.js','favicon.ico','icons','pkg','assets'):
    source=Path('web')/name
    if source.is_dir(): shutil.copytree(source,root/name)
    elif source.is_file(): shutil.copy2(source,root/name)
revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
wasm=root/'pkg/bevy_open_rts_bg.wasm'
(root/'build.json').write_text(json.dumps({'revision':revision,'engine_inputs_sha256':current_inputs,'wasm_sha256':hashlib.sha256(wasm.read_bytes()).hexdigest(),'wasm_bytes':wasm.stat().st_size},indent=2)+'\n')
(root/'.nojekyll').touch()
generate(root,revision,Path('web/sw.js'))
