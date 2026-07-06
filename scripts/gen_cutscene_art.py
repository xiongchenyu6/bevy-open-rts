import json, sys, time, urllib.request, urllib.parse

BASE = "https://comfy.autolfie.ddns.net"
IP = "101.78.126.6"
import socket, ssl
# Force-resolve the hostname to the office IP (local DNS hijacks it to 127.0.0.1).
real_getaddrinfo = socket.getaddrinfo
def patched(host, *a, **kw):
    if host == "comfy.autolfie.ddns.net":
        host = IP
    return real_getaddrinfo(host, *a, **kw)
socket.getaddrinfo = patched
# SNI still needs the hostname; urllib uses it from the URL. Cert matches hostname.

TOKEN = open(sys.argv[1]).read().strip()
PROMPT = sys.argv[2]
OUT = sys.argv[3]
SEED = int(sys.argv[4]) if len(sys.argv) > 4 else 7

def api(path, data=None):
    req = urllib.request.Request(BASE + path,
        data=json.dumps(data).encode() if data else None,
        headers={"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(req, timeout=60))

wf = {
  "ckpt": {"class_type": "CheckpointLoaderSimple",
           "inputs": {"ckpt_name": "flux1-dev-fp8.safetensors"}},
  "pos": {"class_type": "CLIPTextEncode",
          "inputs": {"text": PROMPT, "clip": ["ckpt", 1]}},
  "guide": {"class_type": "FluxGuidance",
            "inputs": {"conditioning": ["pos", 0], "guidance": 3.5}},
  "neg": {"class_type": "CLIPTextEncode",
          "inputs": {"text": "", "clip": ["ckpt", 1]}},
  "latent": {"class_type": "EmptySD3LatentImage",
             "inputs": {"width": 1344, "height": 768, "batch_size": 1}},
  "sample": {"class_type": "KSampler",
             "inputs": {"model": ["ckpt", 0], "positive": ["guide", 0], "negative": ["neg", 0],
                        "latent_image": ["latent", 0], "seed": SEED, "steps": 22,
                        "cfg": 1.0, "sampler_name": "euler", "scheduler": "simple", "denoise": 1.0}},
  "decode": {"class_type": "VAEDecode",
             "inputs": {"samples": ["sample", 0], "vae": ["ckpt", 2]}},
  "save": {"class_type": "SaveImage",
           "inputs": {"images": ["decode", 0], "filename_prefix": "rts_cutscene"}},
}
r = api("/prompt", {"prompt": wf})
pid = r["prompt_id"]
print("queued", pid, flush=True)
for _ in range(240):
    time.sleep(5)
    h = api(f"/history/{pid}")
    if pid in h and h[pid].get("outputs"):
        imgs = h[pid]["outputs"]["save"]["images"]
        img = imgs[0]
        q = urllib.parse.urlencode({"filename": img["filename"], "subfolder": img.get("subfolder",""), "type": img["type"]})
        req = urllib.request.Request(f"{BASE}/view?{q}", headers={"Authorization": f"Bearer {TOKEN}"})
        data = urllib.request.urlopen(req, timeout=120).read()
        open(OUT, "wb").write(data)
        print("saved", OUT, len(data), flush=True)
        sys.exit(0)
print("TIMEOUT", flush=True)
sys.exit(1)

# Usage:
#   TOKEN=$(cd ~/Documents/github/autolife/nixos && sops -d --extract '["comfyui"]["bearer-token"]' secrets/sg-office.yaml)
#   echo "$TOKEN" > /tmp/comfy_token
#   python3 scripts/gen_cutscene_art.py /tmp/comfy_token "<prompt>" out.png [seed]
# Server: ComfyUI (flux1-dev-fp8) behind https://comfy.autolfie.ddns.net
# NOTE: local DNS may hijack the hostname to 127.0.0.1 — the script pins the
# office IP via a getaddrinfo patch. Convert results with:
#   magick out.png -resize 1280x -quality 84 assets/campaign/mission_NN.jpg
