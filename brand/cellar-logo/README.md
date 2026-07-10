# Cellar logo pack

This folder contains the editable and production-ready exports for the Cellar icon.

## Primary files

- `svg/cellar-icon-gradient.svg`: editable vector master with the purple-to-blue finish.
- `png/cellar-icon-gradient-1024.png`: transparent high-resolution raster master.
- `macos/cellar-app-icon-1024.png`: monochrome rounded app tile used to generate platform icons.
- `macos/cellar-icon.icns`: macOS application icon.
- `windows/cellar-icon.ico`: Windows application icon.
- `favicon/favicon.ico`: multi-resolution website favicon.

## Variants

The `svg/` and `png/` folders include:

- Gradient on transparent.
- Black on transparent.
- White on transparent.
- Black on white.
- White on black.

Transparent gradient PNGs are supplied at 16, 32, 48, 64, 128, 180, 192, 256, 512, and 1024 pixels. Monochrome variants are supplied at 128, 256, 512, and 1024 pixels.

## Favicons

The `favicon/` folder includes ICO, SVG, and PNG exports at 16, 32, 48, 128, 180, 192, 256, and 512 pixels. The 16px export is pixel-hinted for crisp browser-tab rendering; larger sizes retain antialiasing.

## Source and provenance

- `source/cellar-logo-reference.png` is the supplied reference artwork.
- `source/cellar-icon-chroma.png` is the intermediate background-extraction render.
- `source/cellar-app-icon-safe.svg` is the editable macOS application-icon master, including the optical safe area.
- `source/app-icon-mask.svg` preserves the full-size rounded tile template.
- `source/IMAGEGEN-PROMPT.md` records the exact reconstruction prompt.

The current SVG masters are measured directly from the supplied reference silhouette. A 1024px raster comparison covers 98.4% of the same silhouette, with the remaining difference limited to antialiasing and cleaned curve transitions. The older chroma extraction and its prompt are retained only as provenance; production exports no longer use that generated geometry.

## Usage

- Use black on light surfaces and white on dark surfaces for website and platform branding.
- Reserve the gradient version for in-app logo treatments that deliberately follow the user's selected accent.
- Keep the 96 px transparent inset in the macOS master; it brings the Dock icon into line with Apple's optical sizing.
- Keep clear space around the mark equal to at least half the width of one database layer.
