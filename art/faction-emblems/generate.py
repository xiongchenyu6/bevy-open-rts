"""Original faction crests, authored as vector geometry without image inputs."""
from pathlib import Path
import hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parent
DESIGNS={
'alliance': '<path fill="#79d9e5" d="M18 24h92L99 80 64 112 29 80Z"/><path fill="#152831" d="M30 35h68L88 74 64 97 40 74Z"/><path fill="#f1d37d" d="m64 39 7 16 18 2-13 12 3 18-15-9-15 9 3-18-13-12 18-2Z"/><path fill="#c9f3f1" d="M8 42h19l5 19H17ZM120 42h-19l-5 19h15Z"/>',
'demon': '<path fill="#ed7756" d="M13 17 40 36 64 24 88 36 115 17 105 57 91 79 64 112 37 79 23 57Z"/><path fill="#44252c" d="m33 42 20 7 11-11 11 11 20-7-7 29-24 27-24-27Z"/><path fill="#f5ce78" d="M64 47c-2 17-15 17-15 31 0 8 7 14 15 14s15-6 15-14c0-8-5-12-9-16 0 9-6 10-7 6 4-7 4-13 1-21Z"/>',
'chaos': '<path fill="none" stroke="#bc94ed" stroke-width="10" stroke-linejoin="round" d="M52 15 20 34v42l32 19m24 18 32-19V52L76 33"/><path fill="#74e2d0" d="m64 19 23 35-17 14 15 34-23-22-19 9 9-35-13-8Z"/><path fill="#f3edff" d="m66 39 8 16-14 13-2-12Z"/>'}
def svg(body,size=128):return f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 128 128">{body}</svg>\n'
if __name__=='__main__':
 out=Path(sys.argv[1]);out.mkdir(exist_ok=False);records=[]
 for name,body in DESIGNS.items():
  source=out/(name+'.svg');source.write_text(svg(body));png=source.with_suffix('.png');subprocess.run(['resvg',str(source),str(png)],check=True)
  records.append({'faction':name,'svg_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'png_sha256':hashlib.sha256(png.read_bytes()).hexdigest()})
 cells=[]
 for index,(name,body) in enumerate(DESIGNS.items()):
  x=index*230
  for y,color in [(0,'#101b21'),(200,'#e8edf1')]:
   cells.append(f'<rect x="{x}" y="{y}" width="230" height="200" fill="{color}"/><g transform="translate({x+51},{y+12})">{body}</g>')
   for col,size in enumerate([16,24,32]):cells.append(f'<g transform="translate({x+30+col*62},{y+152}) scale({size/128})">{body}</g>')
 sheet=out/'review-sheet.svg';sheet.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="690" height="400">'+''.join(cells)+'</svg>');subprocess.run(['resvg',str(sheet),str(sheet.with_suffix('.png'))],check=True)
 (out/'manifest.json').write_text(json.dumps({'scope':'Original repository-authored vector candidates, not runtime replacements','authoring_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rasterizer':subprocess.check_output(['resvg','--version'],text=True).strip(),'records':records},indent=2)+'\n')
