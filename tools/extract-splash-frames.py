#!/usr/bin/env python3
"""
extract-splash-frames.py — LunaOS boot splash frame extractor
Uses OpenCV (cv2) — no ffmpeg required.

Usage:
    python tools/extract-splash-frames.py [input.mp4] [output-dir]

Defaults:
    input  = assets/MahinaReveal.mp4
    output = assets/splash-frames/
"""
import cv2
import os
import sys
import shutil

SRC  = os.path.join("assets", "MahinaReveal.mp4")
DST  = os.path.join("assets", "splash-frames")
PNG_COMPRESSION = 6   # 0=fastest/largest, 9=slowest/smallest

def main():
    src = sys.argv[1] if len(sys.argv) > 1 else SRC
    dst = sys.argv[2] if len(sys.argv) > 2 else DST

    cap = cv2.VideoCapture(src)
    if not cap.isOpened():
        print(f"ERROR: Cannot open '{src}'")
        sys.exit(1)

    native_fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
    total      = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    w          = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    h          = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    duration   = total / native_fps

    print(f"Source : {src}")
    print(f"         {w}x{h} @ {native_fps:.2f}fps  {duration:.2f}s  ({total} frames)")
    print(f"Output : {dst}/")
    print()

    if os.path.exists(dst):
        shutil.rmtree(dst)
    os.makedirs(dst)

    idx = 1
    while True:
        ret, frame = cap.read()
        if not ret:
            break

        # cv2 reads BGR. cv2.imwrite saves PNG as standard RGB — colors preserved exactly.
        # No channel swapping, no color alteration.
        path = os.path.join(dst, f"frame{idx:04d}.png")
        cv2.imwrite(path, frame, [cv2.IMWRITE_PNG_COMPRESSION, PNG_COMPRESSION])
        idx += 1

        pct = int((idx - 1) / total * 100)
        print(f"\r  [{pct:3d}%] {idx-1}/{total}", end="", flush=True)

    cap.release()
    total_out = idx - 1

    # Write metadata so luna-splash knows frame dimensions at runtime
    with open(os.path.join(dst, "meta.txt"), "w") as f:
        f.write(f"frames={total_out}\n")
        f.write(f"width={w}\n")
        f.write(f"height={h}\n")
        f.write(f"fps={int(native_fps)}\n")

    size_mb = sum(
        os.path.getsize(os.path.join(dst, fn))
        for fn in os.listdir(dst) if fn.endswith(".png")
    ) / 1_000_000

    print(f"\n\nDone! {total_out} frames @ {w}x{h}")
    print(f"Total : {size_mb:.1f} MB  ({dst}/)")

if __name__ == "__main__":
    main()
