"""Makes the Store package's images from Kuchu's icon (icons/source.png), at the sizes the manifest names them:
python app/src-tauri/windows/store/assets.py (needs Pillow). A tile keeps the pet clear of its edges; the small icons
fill theirs."""
from pathlib import Path

from PIL import Image

HERE = Path(__file__).parent
SOURCE = Image.open(HERE.parent.parent / "icons" / "source.png").convert("RGBA")

# Name: (width, height, how much of the height the pet takes).
SIZES = {
    "StoreLogo": (50, 50, 1.0),
    "Square44x44Logo": (44, 44, 1.0),
    "Square150x150Logo": (150, 150, 0.7),
    "Wide310x150Logo": (310, 150, 0.7),
}

out = HERE / "Assets"
out.mkdir(exist_ok=True)
for name, (width, height, share) in SIZES.items():
    side = round(height * share)
    pet = SOURCE.resize((side, side), Image.LANCZOS)
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    image.paste(pet, ((width - side) // 2, (height - side) // 2), pet)
    image.save(out / f"{name}.png")
    print(name, image.size)
