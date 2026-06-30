#!/usr/bin/env python3
"""Drive the tracked Hunyuan3D replacement queue through ComfyUI.

Default mode is intentionally non-destructive: it validates the queue and the
ComfyUI node inventory, then writes API workflow JSON files that can be reviewed
or submitted later. Pass --submit explicitly to enqueue generation.
"""

from __future__ import annotations

import argparse
import json
import shlex
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_QUEUE = REPO_ROOT / "docs/model-quality/hunyuan3d-queue.json"
DEFAULT_WORKFLOW_DIR = REPO_ROOT / "docs/model-quality/hunyuan3d-workflows"
DEFAULT_RUN_DIR = REPO_ROOT / "docs/model-quality/hunyuan3d-runs"
DEFAULT_REFERENCE_DIR = REPO_ROOT / "docs/model-quality/hunyuan3d-references"
DEFAULT_HARNESS_DIR = REPO_ROOT / "screenshots/model-harness"
DEFAULT_BASE_URL = "http://127.0.0.1:8188"
DEFAULT_REMOTE_COMFY_DIR = "/data/comfyui"
DEFAULT_REMOTE_INPUT_SUBDIR = "bevy-open-rts"
DEFAULT_HY_CHECKPOINT = "hunyuan3d-dit-v2.safetensors"

TENCENT_TEXT_NODE = "TencentTextToModelNode"
REQUIRED_NODE_GROUPS = {
    "text_to_glb": {TENCENT_TEXT_NODE},
    "local_hunyuan_image_to_mesh": {
        "EmptyLatentHunyuan3Dv2",
        "Hunyuan3Dv2Conditioning",
        "VAEDecodeHunyuan3D",
        "VoxelToMesh",
        "SaveGLB",
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate, stage, or submit the Hunyuan3D model replacement queue."
    )
    parser.add_argument("--queue", type=Path, default=DEFAULT_QUEUE)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument(
        "--ssh",
        metavar="HOST",
        help="Run ComfyUI API requests through ssh, e.g. root@101.78.126.6.",
    )
    parser.add_argument("--entity", action="append", help="Only process this entity id; may repeat.")
    parser.add_argument("--limit", type=int, default=0, help="Maximum queue items to process.")
    parser.add_argument("--workflows-dir", type=Path, default=DEFAULT_WORKFLOW_DIR)
    parser.add_argument("--run-dir", type=Path, default=DEFAULT_RUN_DIR)
    parser.add_argument("--reference-dir", type=Path, default=DEFAULT_REFERENCE_DIR)
    parser.add_argument("--harness-dir", type=Path, default=DEFAULT_HARNESS_DIR)
    parser.add_argument(
        "--workflow-mode",
        default="local-image",
        choices=["local-image", "tencent-text"],
        help="local-image uses Hunyuan3D-2 image-to-model; tencent-text uses the paid text-to-model API node.",
    )
    parser.add_argument("--remote-comfy-dir", default=DEFAULT_REMOTE_COMFY_DIR)
    parser.add_argument("--remote-input-subdir", default=DEFAULT_REMOTE_INPUT_SUBDIR)
    parser.add_argument(
        "--use-existing-reference",
        action="store_true",
        help="For local-image mode, keep an existing reference PNG instead of recropping harness output.",
    )
    parser.add_argument("--hy-checkpoint", default=DEFAULT_HY_CHECKPOINT)
    parser.add_argument("--resolution", type=int, default=4096)
    parser.add_argument("--steps", type=int, default=30)
    parser.add_argument("--cfg", type=float, default=5.0)
    parser.add_argument("--octree-resolution", type=int, default=256)
    parser.add_argument("--voxel-threshold", type=float, default=0.6)
    parser.add_argument("--model", default="3.0", choices=["3.0", "3.1"])
    parser.add_argument(
        "--generate-type",
        default="LowPoly",
        choices=["Normal", "LowPoly", "Geometry"],
        help="Tencent node generation mode. LowPoly is only valid with model 3.0.",
    )
    parser.add_argument("--face-count", type=int, default=120000)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--pbr", action="store_true")
    parser.add_argument("--submit", action="store_true", help="Actually enqueue jobs in ComfyUI.")
    parser.add_argument(
        "--poll",
        action="store_true",
        help="After --submit, poll history and download the first GLB-like output if present.",
    )
    parser.add_argument(
        "--download-history",
        type=Path,
        action="append",
        help=(
            "Download the first GLB-like output from an already saved ComfyUI history JSON. "
            "Useful when generation succeeded but the first download attempt failed."
        ),
    )
    parser.add_argument("--timeout", type=float, default=1800.0)
    parser.add_argument("--poll-interval", type=float, default=5.0)
    parser.add_argument("--overwrite", action="store_true")
    return parser.parse_args()


class ComfyClient:
    def __init__(self, base_url: str, ssh_host: str | None) -> None:
        self.base_url = base_url.rstrip("/")
        self.ssh_host = ssh_host

    def request_bytes(
        self,
        method: str,
        path: str,
        data: bytes | None = None,
        headers: dict[str, str] | None = None,
    ) -> bytes:
        url = f"{self.base_url}{path}"
        if self.ssh_host:
            cmd = ["ssh", self.ssh_host, "curl", "-sS", "-X", method]
            for key, value in (headers or {}).items():
                cmd.extend(["-H", f"{key}:{value}"])
            if data is not None:
                cmd.extend(["--data-binary", "@-"])
            cmd.append(shlex.quote(url))
            proc = subprocess.run(
                cmd,
                input=data,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            if proc.returncode != 0:
                raise RuntimeError(proc.stderr.decode("utf-8", errors="replace").strip())
            return proc.stdout

        req = urllib.request.Request(url, method=method, data=data)
        for key, value in (headers or {}).items():
            req.add_header(key, value)
        try:
            with urllib.request.urlopen(req, timeout=30) as response:
                return response.read()
        except urllib.error.URLError as exc:
            raise RuntimeError(str(exc)) from exc

    def request_json(self, method: str, path: str, payload: Any | None = None) -> Any:
        data = None
        headers = None
        if payload is not None:
            data = json.dumps(payload, ensure_ascii=True).encode("utf-8")
            headers = {"Content-Type": "application/json"}
        body = self.request_bytes(method, path, data=data, headers=headers)
        return json.loads(body.decode("utf-8"))


def load_queue(path: Path, entities: set[str] | None, limit: int) -> list[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as fh:
        queue = json.load(fh)
    if entities:
        queue = [item for item in queue if item.get("entity") in entities]
    if limit > 0:
        queue = queue[:limit]
    return queue


def generation_prompt(item: dict[str, Any]) -> str:
    prompt = item["prompt"]
    negative = item.get("negative_prompt")
    if negative:
        prompt = f"{prompt}\nAvoid: {negative}"
    return prompt[:1024]


def tencent_generate_type_inputs(args: argparse.Namespace) -> dict[str, Any]:
    if args.generate_type == "LowPoly":
        if args.model == "3.1":
            raise ValueError("Tencent LowPoly mode is unavailable for model 3.1; use --model 3.0.")
        return {
            "generate_type": "LowPoly",
            "generate_type.polygon_type": "triangle",
            "generate_type.pbr": bool(args.pbr),
        }
    if args.generate_type == "Normal":
        return {
            "generate_type": "Normal",
            "generate_type.pbr": bool(args.pbr),
        }
    return {"generate_type": "Geometry"}


def workflow_for_item(item: dict[str, Any], args: argparse.Namespace) -> dict[str, Any]:
    return {
        "1": {
            "class_type": TENCENT_TEXT_NODE,
            "inputs": {
                "model": args.model,
                "prompt": generation_prompt(item),
                "face_count": args.face_count,
                "seed": args.seed,
                **tencent_generate_type_inputs(args),
            },
        }
    }


def write_workflow(path: Path, workflow: dict[str, Any], overwrite: bool) -> None:
    if path.exists() and not overwrite:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        json.dump({"prompt": workflow}, fh, ensure_ascii=False, indent=2)
        fh.write("\n")


def preflight(client: ComfyClient) -> dict[str, Any]:
    object_info = client.request_json("GET", "/object_info")
    available = set(object_info)
    result = {
        "available_groups": {},
        "missing_groups": {},
        "node_count": len(available),
    }
    for name, nodes in REQUIRED_NODE_GROUPS.items():
        missing = sorted(nodes - available)
        if missing:
            result["missing_groups"][name] = missing
        else:
            result["available_groups"][name] = sorted(nodes)
    return result


def local_image_workflow_for_item(
    item: dict[str, Any],
    args: argparse.Namespace,
    image_name: str,
) -> dict[str, Any]:
    return {
        "1": {
            "class_type": "ImageOnlyCheckpointLoader",
            "inputs": {"ckpt_name": args.hy_checkpoint},
        },
        "2": {
            "class_type": "LoadImage",
            "inputs": {"image": image_name},
        },
        "3": {
            "class_type": "CLIPVisionEncode",
            "inputs": {"clip_vision": ["1", 1], "image": ["2", 0], "crop": "center"},
        },
        "4": {
            "class_type": "Hunyuan3Dv2Conditioning",
            "inputs": {"clip_vision_output": ["3", 0]},
        },
        "5": {
            "class_type": "EmptyLatentHunyuan3Dv2",
            "inputs": {"resolution": args.resolution, "batch_size": 1},
        },
        "6": {
            "class_type": "ModelSamplingAuraFlow",
            "inputs": {"model": ["1", 0], "shift": 1.0},
        },
        "7": {
            "class_type": "KSampler",
            "inputs": {
                "model": ["6", 0],
                "positive": ["4", 0],
                "negative": ["4", 1],
                "latent_image": ["5", 0],
                "seed": args.seed,
                "steps": args.steps,
                "cfg": args.cfg,
                "sampler_name": "euler",
                "scheduler": "normal",
                "denoise": 1.0,
            },
        },
        "8": {
            "class_type": "VAEDecodeHunyuan3D",
            "inputs": {
                "samples": ["7", 0],
                "vae": ["1", 2],
                "num_chunks": 8000,
                "octree_resolution": args.octree_resolution,
            },
        },
        "9": {
            "class_type": "VoxelToMesh",
            "inputs": {
                "voxel": ["8", 0],
                "algorithm": "surface net",
                "threshold": args.voxel_threshold,
            },
        },
        "10": {
            "class_type": "SaveGLB",
            "inputs": {
                "mesh": ["9", 0],
                "filename_prefix": f"{args.remote_input_subdir}/{item['entity']}",
            },
        },
    }


def image_dimensions(path: Path) -> tuple[int, int]:
    proc = subprocess.run(
        ["identify", "-format", "%w %h", str(path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or f"identify failed for {path}")
    width, height = proc.stdout.strip().split()
    return int(width), int(height)


def harness_canvas_geometry(path: Path) -> tuple[int, int, int, int]:
    proc = subprocess.run(
        ["magick", str(path), "-fuzz", "2%", "-trim", "-format", "%w %h %X %Y", "info:"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or f"canvas trim failed for {path}")
    width, height, x, y = proc.stdout.strip().split()
    return int(width), int(height), int(x), int(y)


def parse_cell(cell: str) -> tuple[int, int]:
    parts = cell.replace("r", "").replace("c", "").split()
    if len(parts) != 2:
        raise ValueError(f"unexpected harness cell format: {cell!r}")
    return int(parts[0]), int(parts[1])


def crop_reference_image(item: dict[str, Any], args: argparse.Namespace) -> Path:
    harness = item.get("harness") or {}
    source = args.harness_dir / str(harness.get("screenshot", ""))
    if not source.exists():
        raise FileNotFoundError(f"missing harness screenshot for {item['entity']}: {source}")
    row, col = parse_cell(str(harness.get("cell", "")))
    width, height, canvas_x, canvas_y = harness_canvas_geometry(source)
    cell_w = width // 3
    cell_h = height // 2
    crop_w = int(cell_w * 0.86)
    crop_h = int(cell_h * 0.82)
    crop_x = canvas_x + col * cell_w + (cell_w - crop_w) // 2
    crop_y = canvas_y + row * cell_h + (cell_h - crop_h) // 2
    target = args.reference_dir / f"{item['entity']}.png"
    if target.exists() and (args.use_existing_reference or not args.overwrite):
        return target
    target.parent.mkdir(parents=True, exist_ok=True)
    geometry = f"{crop_w}x{crop_h}+{crop_x}+{crop_y}"
    proc = subprocess.run(
        [
            "magick",
            str(source),
            "-crop",
            geometry,
            "+repage",
            "-fuzz",
            "4%",
            "-trim",
            "+repage",
            "-resize",
            "768x768",
            "-background",
            "black",
            "-gravity",
            "center",
            "-extent",
            "768x768",
            str(target),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or f"magick crop failed for {source}")
    return target


def stage_reference_image(path: Path, args: argparse.Namespace) -> str:
    image_name = f"{args.remote_input_subdir}/{path.name}"
    if not args.ssh:
        return image_name
    remote_dir = f"{args.remote_comfy_dir.rstrip('/')}/input/{args.remote_input_subdir}"
    subprocess.run(["ssh", args.ssh, "mkdir", "-p", remote_dir], check=True)
    subprocess.run(["scp", str(path), f"{args.ssh}:{remote_dir}/{path.name}"], check=True)
    return image_name


def submit_workflow(client: ComfyClient, workflow: dict[str, Any]) -> str:
    response = client.request_json("POST", "/prompt", {"prompt": workflow})
    prompt_id = response.get("prompt_id")
    if not prompt_id:
        raise RuntimeError(f"ComfyUI did not return prompt_id: {response}")
    return prompt_id


def poll_history(client: ComfyClient, prompt_id: str, timeout: float, interval: float) -> dict[str, Any]:
    deadline = time.time() + timeout
    while time.time() < deadline:
        history = client.request_json("GET", f"/history/{urllib.parse.quote(prompt_id)}")
        if prompt_id in history:
            return history[prompt_id]
        time.sleep(interval)
    raise TimeoutError(f"Timed out waiting for ComfyUI prompt {prompt_id}")


def walk_dicts(value: Any) -> list[dict[str, Any]]:
    found: list[dict[str, Any]] = []
    if isinstance(value, dict):
        found.append(value)
        for child in value.values():
            found.extend(walk_dicts(child))
    elif isinstance(value, list):
        for child in value:
            found.extend(walk_dicts(child))
    return found


def downloadable_files(history: dict[str, Any]) -> list[dict[str, str]]:
    files = []
    for node in walk_dicts(history.get("outputs", {})):
        filename = node.get("filename")
        if not isinstance(filename, str):
            continue
        if not filename.lower().endswith((".glb", ".gltf", ".obj", ".fbx", ".stl", ".usdz")):
            continue
        files.append(
            {
                "filename": filename,
                "subfolder": str(node.get("subfolder", "")),
                "type": str(node.get("type", "output")),
            }
        )
    return files


def entity_from_history_path(path: Path) -> str:
    return path.stem.split("-", 1)[0]


def target_for_entity(entity: str, queue: list[dict[str, Any]]) -> Path:
    for item in queue:
        if item.get("entity") == entity:
            return REPO_ROOT / str(item["target_path"])
    return REPO_ROOT / "assets" / "models" / "hunyuan3d" / f"{entity}.glb"


def download_file(client: ComfyClient, file_info: dict[str, str], target: Path, overwrite: bool) -> bool:
    if target.exists() and not overwrite:
        return False
    query = urllib.parse.urlencode(file_info)
    data = client.request_bytes("GET", f"/view?{query}")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    return True


def main() -> int:
    args = parse_args()
    entities = set(args.entity) if args.entity else None
    queue = load_queue(args.queue, entities, args.limit)
    if not queue and not args.download_history:
        print("[hunyuan3d] no queue entries selected", file=sys.stderr)
        return 1

    client = ComfyClient(args.base_url, args.ssh)
    info = preflight(client)
    print(f"[hunyuan3d] ComfyUI nodes: {info['node_count']}")
    for group, nodes in info["available_groups"].items():
        print(f"[hunyuan3d] available {group}: {', '.join(nodes)}")
    for group, nodes in info["missing_groups"].items():
        print(f"[hunyuan3d] missing {group}: {', '.join(nodes)}")
    if args.workflow_mode == "tencent-text" and TENCENT_TEXT_NODE not in info["available_groups"].get(
        "text_to_glb", []
    ):
        print("[hunyuan3d] text-to-GLB node is unavailable; refusing to submit", file=sys.stderr)
        return 2

    if args.download_history:
        for history_path in args.download_history:
            history = json.loads(history_path.read_text(encoding="utf-8"))
            files = downloadable_files(history)
            if not files:
                print(f"[hunyuan3d] no downloadable 3D file found in {history_path}", file=sys.stderr)
                return 3
            entity = next(iter(entities)) if entities and len(entities) == 1 else entity_from_history_path(history_path)
            target = target_for_entity(entity, queue)
            wrote = download_file(client, files[0], target, args.overwrite)
            action = "downloaded" if wrote else "kept existing"
            print(f"[hunyuan3d] {action} {entity}: {target.relative_to(REPO_ROOT)}")
        if not queue:
            return 0

    args.workflows_dir.mkdir(parents=True, exist_ok=True)
    args.run_dir.mkdir(parents=True, exist_ok=True)
    generated = []
    for item in queue:
        if args.workflow_mode == "local-image":
            reference = crop_reference_image(item, args)
            image_name = stage_reference_image(reference, args)
            workflow = local_image_workflow_for_item(item, args, image_name)
            print(f"[hunyuan3d] reference {item['entity']}: {reference.relative_to(REPO_ROOT)}")
        else:
            workflow = workflow_for_item(item, args)
        workflow_path = args.workflows_dir / f"{item['entity']}.json"
        write_workflow(workflow_path, workflow, args.overwrite)
        print(f"[hunyuan3d] workflow {item['entity']}: {workflow_path.relative_to(REPO_ROOT)}")
        if not args.submit:
            continue
        target = REPO_ROOT / item["target_path"]
        prompt_id = submit_workflow(client, workflow)
        generated.append({"entity": item["entity"], "prompt_id": prompt_id, "target": item["target_path"]})
        print(f"[hunyuan3d] submitted {item['entity']}: {prompt_id}")
        if args.poll:
            history = poll_history(client, prompt_id, args.timeout, args.poll_interval)
            history_path = args.run_dir / f"{item['entity']}-{prompt_id}.history.json"
            history_path.write_text(json.dumps(history, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            files = downloadable_files(history)
            if not files:
                print(f"[hunyuan3d] no downloadable 3D file found for {item['entity']}")
                continue
            wrote = download_file(client, files[0], target, args.overwrite)
            action = "downloaded" if wrote else "kept existing"
            print(f"[hunyuan3d] {action} {item['entity']}: {item['target_path']}")

    if generated:
        manifest = args.run_dir / f"submitted-{int(time.time())}.json"
        manifest.write_text(json.dumps(generated, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[hunyuan3d] wrote {manifest.relative_to(REPO_ROOT)}")
    elif not args.submit:
        print("[hunyuan3d] dry run complete; pass --submit to enqueue generation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
