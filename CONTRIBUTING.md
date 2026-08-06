# How to build locally (for developers)

You do not need these steps if you use the installers above.

## 0) One-time prerequisites

### Install toolchains

Install these tools:

- Node.js
- Rust

Make sure the tools work:

```bash
node -v
npm -v
rustc --version
cargo --version
```

---

## 1) Install frontend dependencies

Run:

```bash
npm install
```

Make sure this creates `node_modules/`.

---

## 2) Run the frontend alone (sanity check)

Run:

```bash
npm run dev
```

Make sure you see:

```
Local: http://localhost:5173/
```

Stop with:

```bash
Ctrl+C
```

---

## 3) Confirm Tauri config is correct

Open:

```
src-tauri/tauri.conf.json
```

Make sure you see:

```json
"build": {
  "frontendDist": "../dist",
  "devUrl": "http://localhost:5173",
  "beforeDevCommand": "npm run dev",
  "beforeBuildCommand": "npm run build"
}
```

---

## 4) First Tauri dev run (builds Rust once)

Run:

```bash
npx tauri dev
```

Make sure the app starts. Then stop with:

```bash
Ctrl+C
```

---

## 5) Build the production frontend assets

Run:

```bash
npm run build
```

Make sure you have:

```
dist/index.html
dist/assets/...
```

---

## 6) Build the installable app (release)

Run:

```bash
npx tauri build
```

This makes OS-native outputs under:

```
src-tauri/target/release/bundle/
```

Common locations:

### Windows

- `src-tauri/target/release/bundle/nsis/*.exe`
- `src-tauri/target/release/bundle/msi/*.msi`

### macOS

- `src-tauri/target/release/bundle/macos/*.app`
- `src-tauri/target/release/bundle/dmg/*.dmg`

### Linux

- `src-tauri/target/release/bundle/appimage/*.AppImage`
- `src-tauri/target/release/bundle/deb/*.deb`

Run the built artifact. Make sure it starts without a dev server (and without your terminal).
