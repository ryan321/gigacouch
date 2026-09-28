"""Install Giga Couch brand assets into macOS app bundles. No downloads."""
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "brand"
MARK = BRAND / "mark.png"
LOGO = BRAND / "logo.png"

# Apple iconutil names. Source is brand/mark.png (transparent mint couch).
ICON_SIZES = (
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
)


def require_brand():
    if not MARK.is_file() or not LOGO.is_file():
        raise RuntimeError(f"Brand PNGs missing under {BRAND} (need mark.png and logo.png).")


def plist_icon():
    return {"CFBundleIconFile": "AppIcon"}


# Brand tokens used by the macOS tile icon (see brand/README.md).
MINT = (0x8C, 0xE8, 0xBE)
NAVY = (0x10, 0x17, 0x22)
RAIL = ((0x21, 0x3D, 0x3C), (0x16, 0x27, 0x2D), (0x11, 0x1D, 0x2A))


def clean_mark():
    """Return mark.png redrawn in exactly mint and navy.

    The source has faint white and cyan specks around the arm gaps. They are
    invisible on a transparent Dock but show as a halo on a tile.
    """
    from PIL import Image

    image = Image.open(MARK).convert("RGBA")
    pixels = []
    for red, green, blue, alpha in image.getdata():
        if alpha < 48:
            pixels.append((0, 0, 0, 0))
            continue
        light = (red * 299 + green * 587 + blue * 114) / 1000
        pixels.append((*(MINT if light > 90 else NAVY), alpha))
    image.putdata(pixels)
    return image.crop(image.getbbox())


def make_mac_tile(destination: Path, size: int = 1024) -> Path:
    """Draw the macOS app icon: the couch on a rounded rail-gradient tile.

    Proportions follow Apple's icon grid: an 824px tile on a 1024px canvas,
    corner radius about 185px, with a soft shadow in the margin.
    """
    from PIL import Image, ImageDraw, ImageFilter

    require_brand()
    scale = size / 1024
    inset, radius = round(100 * scale), round(185 * scale)
    box = (inset, inset, size - inset, size - inset)

    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, radius=radius, fill=255)

    gradient = Image.new("RGB", (1, size))
    top, mid, bottom = RAIL
    for y in range(size):
        t = y / (size - 1)
        low, high, f = (top, mid, t / 0.5) if t < 0.5 else (mid, bottom, (t - 0.5) / 0.5)
        gradient.putpixel((0, y), tuple(round(a + (b - a) * f) for a, b in zip(low, high)))
    gradient = gradient.resize((size, size))

    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    shadow_mask = Image.new("L", (size, size), 0)
    offset = round(12 * scale)
    ImageDraw.Draw(shadow_mask).rounded_rectangle(
        (box[0], box[1] + offset, box[2], box[3] + offset), radius=radius, fill=90
    )
    shadow.putalpha(shadow_mask.filter(ImageFilter.GaussianBlur(round(18 * scale))))

    icon = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    icon.alpha_composite(shadow)
    tile = gradient.convert("RGBA")
    tile.putalpha(mask)
    icon.alpha_composite(tile)

    couch = clean_mark()
    width = round(600 * scale)
    height = round(couch.height * width / couch.width)
    couch = couch.resize((width, height), Image.LANCZOS)
    icon.alpha_composite(couch, ((size - width) // 2, (size - height) // 2 + round(10 * scale)))

    destination.parent.mkdir(parents=True, exist_ok=True)
    icon.save(destination)
    return destination


def make_icns(destination: Path, source: Path = MARK) -> Path:
    require_brand()
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="gigacouch-iconset-") as temp:
        iconset = Path(temp) / "AppIcon.iconset"
        iconset.mkdir()
        for name, size in ICON_SIZES:
            result = subprocess.run(
                ["sips", "-z", str(size), str(size), str(source), "--out", str(iconset / name)],
                capture_output=True, text=True,
            )
            if result.returncode:
                raise RuntimeError(result.stderr.strip() or f"sips failed for {name}")
        result = subprocess.run(
            ["iconutil", "-c", "icns", "-o", str(destination), str(iconset)],
            capture_output=True, text=True,
        )
        if result.returncode:
            raise RuntimeError(result.stderr.strip() or "iconutil failed")
    return destination


def install_into_app(contents: Path, icns: Path) -> None:
    require_brand()
    resources = contents / "Resources"
    resources.mkdir(parents=True, exist_ok=True)
    shutil.copy2(icns, resources / "AppIcon.icns")
    shutil.copy2(MARK, resources / "mark.png")
    shutil.copy2(LOGO, resources / "logo.png")
