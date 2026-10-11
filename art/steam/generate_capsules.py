"""Compose original wordmark and generated key art into editable Steam SVG layouts."""
from pathlib import Path
import hashlib
import importlib.util
import json
import struct
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('open_rts_wordmark',ROOT/'art/branding/generate.py')
branding=importlib.util.module_from_spec(spec);spec.loader.exec_module(branding)
ART=ROOT/'art/steam/keyart-v1/keyart.png'
FORMATS=[('header',920,430),('small',462,174),('main',1232,706),('vertical',748,896),('library-capsule',600,900),('library-hero',3840,1240)]
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def generate(out):
    out.mkdir(parents=True,exist_ok=False)
    records=[]
    for name,w,h in FORMATS:
        import os
        art_path=os.path.relpath(ART,out)
        align='xMidYMax' if name=='library-hero' else 'xMidYMid'
        image=f'<image href="{art_path}" width="{w}" height="{h}" preserveAspectRatio="{align} slice"/>'
        if name=='library-hero':
            image_height=w*941/1672
            image=f'<image href="{art_path}" y="{-0.82*(image_height-h)}" width="{w}" height="{image_height}"/>'
        title=''
        if name!='library-hero':
            title_width=w*(.88 if name in ('small','vertical','library-capsule') else .70)
            scale=title_width/630
            x=(w-title_width)/2 if name in ('vertical','library-capsule') else w*.045
            y=h*(.045 if name=='small' else .055)
            # Only shade the title area; retain the original artwork beneath it.
            title=f'<rect width="{w}" height="{110*scale+y+32}" fill="url(#shade)"/><g transform="translate({x},{y}) scale({scale})">{branding.wordmark()}</g>'
        defs='<defs><linearGradient id="shade" x2="0" y2="1"><stop stop-color="#142a32" stop-opacity=".85"/><stop offset="1" stop-color="#142a32" stop-opacity="0"/></linearGradient></defs>'
        svg=out/(name+'.svg');svg.write_text(branding.svg(w,h,defs+image+title))
        png=svg.with_suffix('.png');subprocess.run(['resvg',str(svg),str(png)],check=True)
        actual=struct.unpack('>II',png.read_bytes()[16:24]);assert actual==(w,h)
        records.append({'name':name,'width':w,'height':h,'svg_sha256':digest(svg),'png_sha256':digest(png)})
    # Review title readability at the small Steam browsing size as well as full exports.
    body='<rect width="740" height="580" fill="#18272d"/>'
    for name,x,y,w,h in [('header',10,10,460,215),('small',10,245,462,174),('vertical',500,10,224,269),('library-capsule',500,299,180,270),('small',10,455,231,87),('small',270,455,126,47)]:
        body+=f'<image href="{name}.png" x="{x}" y="{y}" width="{w}" height="{h}"/>'
    sheet=out/'review-sheet.svg';sheet.write_text(branding.svg(740,580,body));subprocess.run(['resvg',str(sheet),str(sheet.with_suffix('.png'))],check=True)
    (out/'manifest.json').write_text(json.dumps({'scope':'Steam capsule composition candidates; not final store approval or gameplay captures','dimensions_source':'https://partner.steamgames.com/doc/store/assets?l=english','checked':'2026-10-12','generator_sha256':digest(Path(__file__)),'keyart_sha256':digest(ART),'wordmark_generator_sha256':digest(ROOT/'art/branding/generate.py'),'rasterizer':subprocess.check_output(['resvg','--version'],text=True).strip(),'records':records,'remaining':['Small-size and crop visual review','Steam-client placement','Final commercial rights review','Actual final gameplay screenshots and trailer']},indent=2)+'\n')
if __name__=='__main__':generate(Path(sys.argv[1]).resolve())
