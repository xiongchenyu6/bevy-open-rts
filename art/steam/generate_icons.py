"""Native SVG compositions for Steam client icons, using existing original artwork."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import struct
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('wordmark',ROOT/'art/branding/generate.py')
branding=importlib.util.module_from_spec(spec);spec.loader.exec_module(branding)
ART=ROOT/'art/steam/keyart-v1/keyart.png'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def generate(out):
    out.mkdir(parents=True,exist_ok=False);records=[]
    for name,size in [('shortcut-icon',256),('app-icon',184)]:
        # Keep the foreground tank rather than reducing the entire battlefield.
        crop=f'<svg width="{size}" height="{size}" viewBox="500 280 620 620"><image href="{os.path.relpath(ART,out)}" width="1672" height="941"/></svg>'
        width=size*.92;scale=width/630
        title=f'<rect width="{size}" height="{110*scale+14}" fill="#142a32" fill-opacity=".82"/><g transform="translate({size*.04},4) scale({scale})">{branding.wordmark()}</g>'
        svg=out/(name+'.svg');svg.write_text(branding.svg(size,size,crop+title));png=svg.with_suffix('.png')
        subprocess.run(['resvg',str(svg),str(png)],check=True)
        assert struct.unpack('>II',png.read_bytes()[16:24])==(size,size)
        records.append({'path':png.name,'width':size,'height':size,'sha256':sha(png),'svg_sha256':sha(svg)})
    jpg=out/'app-icon.jpg'
    subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-n','-cpuflags','0','-i',str(out/'app-icon.png'),'-frames:v','1','-q:v','2','-pix_fmt','yuvj444p','-threads','1',str(jpg)],check=True)
    records.append({'path':jpg.name,'width':184,'height':184,'sha256':sha(jpg)})
    (out/'manifest.json').write_text(json.dumps({'scope':'Steam client-icon candidates; actual client placement and final rights review pending','dimensions_source':'https://partner.steamgames.com/doc/store/assets?l=english','generator_sha256':sha(Path(__file__)),'keyart_sha256':sha(ART),'rasterizer':subprocess.check_output(['resvg','--version'],text=True).strip(),'jpeg_encoder':subprocess.check_output(['ffmpeg','-version'],text=True).splitlines()[0],'records':records},indent=2)+'\n')
if __name__=='__main__':generate(Path(sys.argv[1]).resolve())
