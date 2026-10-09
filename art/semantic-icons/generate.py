"""Original RTS pictograms. Rasterization preserves the authored SVG geometry."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
ICONS = {
    'AttackMove': '<path d="M18 93 55 56M18 72v21h21"/><circle cx="79" cy="45" r="22"/><path d="M79 12v13m0 40v13M46 45h13m40 0h13" class="gold"/>',
    'ChronoRelay': '<circle cx="64" cy="64" r="30"/><path d="M64 43v23l17 10" class="gold"/><path d="M25 40a44 44 0 0 1 77-5m0-16v16H86M103 88a44 44 0 0 1-77 5m0 16V93h16"/>',
    'DeployMode': '<path d="M44 24h40v43H44z" class="metal"/><path d="M44 66 23 88v17m61-39 21 22v17M64 67v38M15 105h17m23 0h18m23 0h17" class="gold"/><path d="m48 34 16 8 16-8"/>',
    'EmpPulse': '<path d="M67 16 42 63h21l-9 49 33-59H66z" class="gold fill"/><path d="M31 34a43 43 0 0 0 0 60m66-60a43 43 0 0 1 0 60M20 22a59 59 0 0 0 0 84m88-84a59 59 0 0 1 0 84"/>',
    'GuardArea': '<path d="M64 20 94 31v31c0 22-16 38-30 47-14-9-30-25-30-47V31z" class="metal"/><path d="M64 39v42M43 60h42" class="gold"/><path d="M17 37V20h17m60 0h17v17M17 91v17h17m60 0h17V91"/>',
    'HoldPosition': '<circle cx="64" cy="25" r="10" class="gold"/><path d="M64 35v71M44 53h40M26 71v12c0 16 17 25 38 25s38-9 38-25V71" class="metal"/><path d="m16 82 10-11 10 11m56 0 10-11 10 11" class="gold"/>',
    'NaniteRepairSwarm': '<path d="m35 17 17 10v20L35 57 18 47V27zm58 0 17 10v20L93 57 76 47V27zm-58 54 17 10v20l-17 10-17-10V81zm58 0 17 10v20l-17 10-17-10V81z"/><path d="M64 49v30M49 64h30" class="gold"/>',
    'OrbitalStrike': '<path d="m30 22 17-7 12 27-17 7zm39 21 17-7 12 27-17 7z" class="metal"/><path d="m53 30 28 27m-23-8-7 13"/><path d="M58 65 71 99M74 58l13 34" class="gold"/><ellipse cx="77" cy="102" rx="29" ry="10"/><path d="m41 90 8 6m54-8-8 8"/>',
    'Paradrop': '<path d="M19 51a45 39 0 0 1 90 0H19zm0 0 31 41m59-41L78 92M43 51l12 41m30-41L73 92"/><path d="M50 92h28v20H50z" class="gold"/><path d="M43 50c0-20 9-36 21-38 12 2 21 18 21 38" class="metal"/>',
    'Patrol': '<path d="M28 41c12-24 45-32 68-12l10 10m0-22v22H84M100 87c-12 24-45 32-68 12L22 89m0 22V89h22"/><circle cx="28" cy="64" r="8" class="gold fill"/><circle cx="100" cy="64" r="8" class="gold fill"/>',
    'RadarSweep': '<circle cx="64" cy="64" r="43"/><circle cx="64" cy="64" r="25" class="metal"/><path d="M64 21v43l31-30M21 64h86M64 64v43"/><path d="m64 64 31-30a43 43 0 0 1 12 30z" class="beam"/><circle cx="42" cy="80" r="5" class="gold fill"/><circle cx="85" cy="49" r="5" class="gold fill"/>',
    'RallyPoint': '<path d="M38 19v84m0-81h51L78 40l11 19H38" class="gold"/><ellipse cx="43" cy="106" rx="27" ry="9"/><path d="M91 77v26m-11-10 11 10 11-10"/>',
    'Repair': '<path d="M83 17a22 22 0 0 0-24 29L23 82a12 12 0 0 0 17 17l36-36a22 22 0 0 0 30-23L91 53 76 38z" class="metal"/><path d="m77 80 25 26M90 68l16 16" class="gold"/><circle cx="32" cy="91" r="3" class="gold fill"/>',
    'Scatter': '<circle cx="64" cy="64" r="11" class="gold fill"/><path d="m48 48-26-26m0 20V22h20m38 26 26-26m-20 0h20v20M48 80l-26 26m20 0H22V86m58-6 26 26m0-20v20H86"/>',
    'SellStructure': '<path d="m20 59 28-27 27 27m-48-6v44h43V53" class="metal"/><path d="M39 97V73h17v24"/><circle cx="90" cy="89" r="23" class="gold"/><path d="M90 76v26m-8-20h12m-8 13h12" class="gold"/>',
    'ShieldOverdrive': '<path d="M64 17 99 30v34c0 22-18 38-35 49-17-11-35-27-35-49V30z"/><path d="m69 36-20 31h17l-8 25 25-35H66z" class="gold fill"/><path d="M18 49v29m92-29v29" class="metal"/>',
    'StopCommand': '<path d="M43 17h42l26 26v42l-26 26H43L17 85V43z" class="metal"/><path d="M48 47h32v34H48z" class="gold fill"/>',
    'StrategicMissile': '<path d="M84 18c-26 0-45 19-48 43l18 18c24-3 43-22 43-48z" class="metal"/><path d="M39 53 23 58 16 81l21-7m37 1-5 21-23 7 5-17"/><circle cx="72" cy="43" r="8" class="gold"/><path d="m35 87-17 23m29-20-9 20" class="gold"/>',
    'TechLocked': '<path d="m28 22 15 0m42 0h15M22 28v15m0 42v15m78 6H85m-42 0H28m78-6V85m0-42V28"/><path d="M45 60V43a19 19 0 0 1 38 0v17" class="metal"/><path d="M36 60h56v46H36z" class="gold"/><circle cx="64" cy="79" r="5" class="gold fill"/><path d="M64 84v10" class="gold"/>',
    'WeatherStorm': '<path d="M29 69a18 18 0 0 1-1-36 27 27 0 0 1 50-9 22 22 0 0 1 21 43H78" class="metal"/><path d="m66 48-18 35h18l-12 31 30-42H66z" class="gold fill"/><path d="m28 82-7 17m76-17-7 17"/>',
}

STYLE = '.gold{stroke:#e8c574}.metal{stroke:#cbd8d6}.fill{fill:#e8c574;stroke-width:2}.beam{fill:#70c4ca;fill-opacity:.22;stroke:none}'

def svg(body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><style>{STYLE}</style><g fill="none" stroke="#70c4ca" stroke-width="5" stroke-linecap="round" stroke-linejoin="round">{body}</g></svg>\n'


if __name__ == '__main__':
    destination = ROOT / (sys.argv[1] if len(sys.argv) > 1 else 'candidates-v1')
    destination.mkdir(exist_ok=False)
    records = []
    for name, body in ICONS.items():
        source = destination / (name + '.svg')
        source.write_text(svg(body))
        image = source.with_suffix('.png')
        subprocess.run(['resvg', str(source), str(image)], check=True)
        records.append({'id': name, 'source': source.name, 'runtime_candidate': image.name,
                        'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                        'png_sha256': hashlib.sha256(image.read_bytes()).hexdigest()})
    cells = []
    for index, (name, body) in enumerate(ICONS.items()):
        x, y = (index % 5) * 180, (index // 5) * 152
        cells.append(f'<g transform="translate({x + 54},{y + 8}) scale(.5625)" fill="none" stroke="#70c4ca" stroke-width="5" stroke-linecap="round" stroke-linejoin="round">{body}</g><text x="{x+90}" y="{y+112}" text-anchor="middle" fill="#e8eae7" font-family="WenQuanYi Micro Hei" font-size="13">{name}</text>')
    sheet = destination / 'contact-sheet.svg'
    sheet.write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="900" height="608" viewBox="0 0 900 608"><style>{STYLE}</style><rect width="900" height="608" fill="#111c20"/>{"".join(cells)}</svg>\n')
    label_font = ROOT.parents[1] / 'assets/fonts/wqy-microhei-ui.ttf'
    subprocess.run(['resvg', '--use-font-file', str(label_font), str(sheet), str(sheet.with_suffix('.png'))], check=True)
    (destination / 'provenance.json').write_text(json.dumps({
        'scope': 'Original repository-authored vector pictograms; candidates only, no runtime replacement',
        'authoring_source': '../generate.py', 'authoring_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'rasterization': 'resvg SVG-to-PNG; no third-party image inputs',
        'rasterizer_version': subprocess.check_output(['resvg', '--version'], text=True).strip(),
        'label_font_scope': 'Licensed repository font used only in review sheet; individual icons contain no text',
        'review': 'Pending 72px contact-sheet and actual HUD inspection', 'icons': records,
    }, indent=2) + '\n')
