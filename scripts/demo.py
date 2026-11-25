#!/usr/bin/env python3
"""Record the README's demo GIFs by actually running womm.

Each scene types commands into a pseudo-terminal, feeds the real output through
a terminal emulator (pyte), and renders frames with Pillow. Long pauses (the
garbage collector) are compressed so the GIFs stay short.

    python3 -m venv .venv && .venv/bin/pip install pyte pillow fonttools
    cargo build --release
    .venv/bin/python scripts/demo.py            # all scenes
    .venv/bin/python scripts/demo.py hero ship  # some scenes

Output: docs/demo/<scene>.gif. Requires macOS fonts (Menlo, Apple Color Emoji).
"""

import os
import pty
import select
import shutil
import struct
import sys
import tempfile
import time
import fcntl
import termios
from pathlib import Path

import pyte
from fontTools.ttLib import TTCollection, TTFont
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
WOMM = ROOT / "target" / "release" / "womm"
OUT = ROOT / "docs" / "demo"

COLS, ROWS = 110, 32
SIZE = 14
PAD = 14
BG = (30, 30, 46)
FG = (205, 214, 244)
PROMPT = "\x1b[38;5;114m❯\x1b[0m "
PALETTE = {
    "black": (69, 71, 90), "red": (243, 139, 168), "green": (166, 227, 161),
    "brown": (249, 226, 175), "yellow": (249, 226, 175), "blue": (137, 180, 250),
    "magenta": (245, 194, 231), "cyan": (148, 226, 213), "white": (186, 194, 222),
    "brightblack": (88, 91, 112), "brightred": (243, 139, 168), "brightgreen": (166, 227, 161),
    "brightyellow": (249, 226, 175), "brightblue": (137, 180, 250),
    "brightmagenta": (245, 194, 231), "brightcyan": (148, 226, 213), "brightwhite": (230, 233, 245),
}

MENLO = "/System/Library/Fonts/Menlo.ttc"
FALLBACKS = [
    "/System/Library/Fonts/Apple Symbols.ttf",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
]
EMOJI = "/System/Library/Fonts/Apple Color Emoji.ttc"

regular = ImageFont.truetype(MENLO, SIZE, index=0)
bold = ImageFont.truetype(MENLO, SIZE, index=1)
CW = round(regular.getlength("M"))
CH = round(SIZE * 1.3)
menlo_cmap = TTCollection(MENLO).fonts[0].getBestCmap()
fallbacks = [(ImageFont.truetype(p, SIZE), TTFont(p).getBestCmap()) for p in FALLBACKS]
emoji_font = ImageFont.truetype(EMOJI, 160)
_emoji_cache = {}


def emoji(ch):
    if ch not in _emoji_cache:
        img = Image.new("RGBA", (180, 180), (0, 0, 0, 0))
        ImageDraw.Draw(img).text((0, 0), ch, font=emoji_font, embedded_color=True)
        img = img.crop(img.getbbox() or (0, 0, 1, 1))
        side = CH - 2
        _emoji_cache[ch] = img.resize((side, side), Image.LANCZOS)
    return _emoji_cache[ch]


def color(name, default):
    if name == "default":
        return default
    if name in PALETTE:
        return PALETTE[name]
    if len(name) == 6:
        return tuple(int(name[i:i + 2], 16) for i in (0, 2, 4))
    return default


def render(screen):
    img = Image.new("RGB", (COLS * CW + 2 * PAD, ROWS * CH + 2 * PAD + 20), BG)
    d = ImageDraw.Draw(img)
    for i, c in enumerate([(243, 139, 168), (249, 226, 175), (166, 227, 161)]):
        d.ellipse((PAD + i * 18, 8, PAD + i * 18 + 10, 18), fill=c)
    top = PAD + 16
    for y in range(ROWS):
        line = screen.buffer[y]
        for x in range(COLS):
            cell = line[x]
            ch = cell.data
            if not ch or ch == " ":
                if cell.bg != "default":
                    d.rectangle((PAD + x * CW, top + y * CH, PAD + (x + 1) * CW, top + (y + 1) * CH), fill=color(cell.bg, BG))
                continue
            px, py = PAD + x * CW, top + y * CH
            if cell.bg != "default":
                d.rectangle((px, py, px + CW, py + CH), fill=color(cell.bg, BG))
            fg = color(cell.fg, FG)
            cp = ord(ch[0])
            if cp >= 0x1F000 or cp in (0x2728, 0x26A0):
                img.paste(emoji(ch[0]), (px, py + 1), emoji(ch[0]))
                continue
            font = bold if cell.bold else regular
            if cp not in menlo_cmap:
                font = next((f for f, cmap in fallbacks if cp in cmap), regular)
            d.text((px, py + 2), ch, font=font, fill=fg)
    return img


class Scene:
    def __init__(self, name, cwd, env):
        self.name, self.cwd, self.env = name, cwd, env
        self.screen = pyte.Screen(COLS, ROWS)
        self.stream = pyte.ByteStream(self.screen)
        self.frames = []

    def snap(self, ms):
        if ms <= 0:
            return
        img = render(self.screen)
        if self.frames and self.frames[-1][0].tobytes() == img.tobytes():
            self.frames[-1] = (img, self.frames[-1][1] + ms)
        else:
            self.frames.append((img, ms))

    def say(self, text, hold=900):
        """A comment line, typed like a human would."""
        self.feed(PROMPT.encode())
        self.type(f"# {text}", colour="38;5;245")
        self.feed(b"\r\n")
        self.snap(hold)

    def type(self, text, colour=None):
        if colour:
            self.feed(f"\x1b[{colour}m".encode())
        for ch in text:
            self.feed(ch.encode())
            self.snap(28)
        if colour:
            self.feed(b"\x1b[0m")

    def feed(self, data):
        self.stream.feed(data)

    def run(self, cmd, answer=None, max_gap=0.45, hold=600):
        """Type `cmd`, run it for real, and record its output with its timing."""
        self.feed(PROMPT.encode())
        self.snap(350)
        self.type(cmd)
        self.snap(350)
        self.feed(b"\r\n")
        args = cmd.split()
        if args[0] == "womm":
            args[0] = str(WOMM)
        pid, fd = pty.fork()
        if pid == 0:
            os.chdir(self.cwd)
            os.execvpe(args[0], args, self.env)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        chunks, start, answered = [], time.time(), False
        while True:
            r, _, _ = select.select([fd], [], [], 0.05)
            if r:
                try:
                    data = os.read(fd, 65536)
                except OSError:
                    break
                if not data:
                    break
                chunks.append((time.time() - start, data))
                if answer and not answered and b"[y/N]" in data:
                    time.sleep(0.8)
                    os.write(fd, answer.encode() + b"\n")
                    answered = True
        os.waitpid(pid, 0)
        # Replay with the original rhythm, but never pause longer than max_gap.
        last = 0.0
        for t, data in chunks:
            self.snap(int(min(max(t - last, 0.0), max_gap) * 1000))
            self.feed(data.replace(b"\n", b"\r\n").replace(b"\r\r\n", b"\r\n"))
            last = t
        self.snap(hold)

    def save(self, hold=3000):
        self.snap(hold)
        OUT.mkdir(parents=True, exist_ok=True)
        imgs = [f[0].quantize(colors=64, method=Image.Quantize.FASTOCTREE) for f in self.frames]
        durations = [max(20, f[1]) for f in self.frames]
        path = OUT / f"{self.name}.gif"
        imgs[0].save(path, save_all=True, append_images=imgs[1:], duration=durations, loop=0, optimize=True, disposal=1)
        print(f"{path.relative_to(ROOT)}: {len(imgs)} frames, {sum(durations) / 1000:.1f}s, {path.stat().st_size // 1024} KB")


def workspace():
    tmp = Path(tempfile.mkdtemp(prefix="womm-demo-"))
    home = tmp / "home"
    home.mkdir()
    shutil.copy(ROOT / "examples" / "synergy.womm", tmp / "synergy.womm")
    env = {
        "PATH": f"{WOMM.parent}:{Path(shutil.which('python3')).parent}:/usr/bin:/bin",
        "HOME": str(home),
        "WOMM_HOME": "home",  # relative, so no long temp paths end up in the GIFs
        "TERM": "xterm-256color",
        "LANG": "en_US.UTF-8",
    }
    # Create the machine key off camera.
    os.system(f"cd {tmp} && WOMM_HOME=home WOMM_FAST=1 {WOMM} parse synergy.womm >/dev/null 2>&1 && rm -rf synergy.kludged")
    return tmp, env


def built(tmp, env, opt="-O1"):
    """Parse, build and link off camera, quickly."""
    fast = " ".join(f"{k}='{v}'" for k, v in {**env, "WOMM_FAST": "1"}.items())
    os.system(f"cd {tmp} && env {fast} {WOMM} parse synergy.womm {opt} >/dev/null && env {fast} {WOMM} build synergy.kludged >/dev/null && env {fast} {WOMM} parsy-builder synergy.womm.md >/dev/null")


def hero():
    tmp, env = workspace()
    s = Scene("hero", tmp, env)
    s.run("cat synergy.womm", hold=900)
    s.run("womm parse synergy.womm -O1", hold=900)
    s.run("womm build -W no synergy.kludged", max_gap=0.35, hold=700)
    s.run("womm parsy-builder synergy.womm.md", hold=900)
    s.run("./synergy --womm-no-review", max_gap=0.06, hold=400)
    s.save(3500)


def build():
    tmp, env = workspace()
    os.system(f"cd {tmp} && WOMM_HOME=home WOMM_FAST=1 {WOMM} parse synergy.womm -O2 >/dev/null")
    s = Scene("build", tmp, env)
    s.run("womm build synergy.kludged", max_gap=0.5, hold=800)
    s.save(3000)


def run_exe():
    tmp, env = workspace()
    built(tmp, env)
    s = Scene("run", tmp, env)
    s.run("./synergy", max_gap=0.05, hold=600)
    s.save(4000)


def visualise():
    tmp, env = workspace()
    built(tmp, env)
    s = Scene("visualise", tmp, env)
    s.run("womm visualise synergy.womm.md", hold=1200)
    s.run("python3 synergy.nonet.py", hold=600)
    s.run("grep -A3 womm:extra:begin synergy.womm.md", hold=600)
    s.say("bit for bit. the most useful thing womm does.")
    s.save(3000)


def blame():
    tmp, env = workspace()
    s = Scene("blame", tmp, env)
    s.run("womm blame synergy.womm", hold=600)
    s.save(4000)


def ship():
    tmp, env = workspace()
    os.system(f"cd {tmp} && WOMM_HOME=home {WOMM} pack-store {ROOT / 'packages'} home/packages.tar >/dev/null")
    s = Scene("ship", tmp, env)
    s.run("womm ship nonet", answer="n", hold=1200)
    s.say("there is no command that lists packages. you found this one yourself.", hold=1500)
    s.save(3000)


SCENES = {"hero": hero, "build": build, "run": run_exe, "visualise": visualise, "blame": blame, "ship": ship}

if __name__ == "__main__":
    for name in sys.argv[1:] or SCENES:
        SCENES[name]()
