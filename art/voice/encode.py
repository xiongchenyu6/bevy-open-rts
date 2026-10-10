#!/usr/bin/env python3
"""Encode portable original voice sources into a new candidate tree; never auto-install."""
import argparse,hashlib,json,math,shutil,struct,subprocess
from pathlib import Path

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def decode(tool,path):
    data=subprocess.check_output([tool,'-hide_banner','-loglevel','error','-threads','1','-i',str(path),'-ac','1','-ar','24000','-f','f32le','pipe:1'])
    values=struct.unpack('<'+'f'*(len(data)//4),data)
    if not values or not all(math.isfinite(x) for x in values):raise ValueError(f'Invalid decoded PCM: {path}')
    clipped=sum(abs(x)>=1 for x in values)
    if clipped:raise ValueError(f'Clipped decoded PCM: {path}')
    return {'sha256':hashlib.sha256(data).hexdigest(),'frames':len(values),'sample_rate':24000,'channels':1,'peak_float':max(abs(x) for x in values),'clipped_samples':clipped}

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);parser.add_argument('--ffmpeg',default=shutil.which('ffmpeg'));parser.add_argument('--check-inputs',action='store_true');args=parser.parse_args()
    base=Path(__file__).resolve().parent;manifest=json.loads((base/'manifest.json').read_text())
    for row in manifest['files']:
        if sha(base/row['source'])!=row['source_sha256']:raise ValueError(f"Changed source: {row['source']}")
    for language in ['english','chinese']:
        events=[e for e in manifest['events'] if e['language']==language]
        if len(events)!=17 or len({e['path'] for e in events})!=16:raise ValueError('Incomplete event mapping')
        files={r['runtime_relative_path'] for r in manifest['files'] if r['language']==language}
        if {e['path'] for e in events}!=files:raise ValueError('Event references an absent candidate')
    if args.check_inputs:
        print(json.dumps({'sources_verified':32,'event_mappings':34,'unique_per_language':16,'runtime_modified':False}));return
    if args.output is None or args.ffmpeg is None:parser.error('--output and an available ffmpeg are required')
    version=subprocess.check_output([args.ffmpeg,'-version'],text=True).splitlines()[0]
    if version!=manifest['tool_version']:raise ValueError(f'Use pinned encoder version: {manifest["tool_version"]}')
    args.output.mkdir(parents=True,exist_ok=False);results=[]
    for row in manifest['files']:
        source=base/row['source'];destination=args.output/row['runtime_relative_path'];destination.parent.mkdir(parents=True,exist_ok=True)
        command=[args.ffmpeg,'-hide_banner','-loglevel','error','-nostdin','-i',str(source),'-map_metadata','-1','-ac','1','-ar','24000','-c:a','libvorbis','-q:a','5','-threads','1',str(destination)]
        subprocess.run(command,check=True);pcm=decode(args.ffmpeg,destination)
        expected=row['expected_decoded_ogg_pcm']
        if pcm['sha256']!=expected['sha256'] or pcm['frames']!=expected['frames']:raise ValueError(f"Decoded PCM differs: {row['runtime_relative_path']}")
        results.append({'path':row['runtime_relative_path'],'source_sha256':row['source_sha256'],'sha256':sha(destination),'bytes':destination.stat().st_size,'decoded_pcm':pcm,'command':command})
    report={'scope':'Candidate-only output; runtime integration/listening unverified','tool_version':version,'tool_sha256':sha(Path(args.ffmpeg).resolve()),'encoder_script_sha256':sha(Path(__file__)),'manifest_sha256':sha(base/'manifest.json'),'files':results,'events':manifest['events'],'human_listening_accepted':False,'tone_accepted':False,'runtime_installed':False}
    (args.output/'encoding-manifest.json').write_text(json.dumps(report,ensure_ascii=False,indent=2))
    print(json.dumps({'encoded':len(results),'decoded_pcm_reference_match':32,'runtime_installed':False}))
if __name__=='__main__':main()
