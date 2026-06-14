# Application icon

Drop your application icon here as:

```
assets/app.ico
```

`build.rs` embeds this icon into the Windows executable at build time (it also
embeds version info either way). If no `app.ico` is present, the build still
succeeds and only the version info is embedded; you will see a `cargo:warning`
reminding you the icon is missing.

The same icon is used by the Inno Setup installer (`installer/setup.iss`) for
the Start Menu / desktop shortcuts, again only if the file exists.

## Recommended format

- A real Windows `.ico` file (not a renamed `.png`).
- Include multiple sizes in the one `.ico`: 16x16, 32x32, 48x48, and 256x256.
- On Windows you can create one from a PNG with ImageMagick:

  ```sh
  magick icon.png -define icon:auto-resize=16,32,48,256 app.ico
  ```

This file is a placeholder note; no binary icon is committed so that no
unrelated image ships with the source.

# UI font (`ui-font.ttf`)

`ui-font.ttf` is the proportional UI font, embedded into the binary at build
time with `include_bytes!` (see `apply_fonts` in `src/main.rs`). It must fully
cover the Turkish Latin letters, since Turkish is the default interface
language and egui's built-in font does not render them reliably.

The committed file is Noto Sans Regular (SIL Open Font License). To swap in a
different font (for example Inter), replace `ui-font.ttf` with another
TrueType (`.ttf`) file that covers Turkish and rebuild. Keep the file name so
no code change is needed.
