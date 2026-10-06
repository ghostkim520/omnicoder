# OmniCoder — GitHub upload instructions

The repository `github.com/fqhd/omnicoder` does not exist yet or isn't accessible with current auth. Push requires authentication (no SSH key set, no `gh` CLI). Choose one option:

## Option 1: Create repo on GitHub (browser), then push via HTTPS with PAT
1. Go to https://github.com/new → create repo `omnicoder` (public)
2. Get a Personal Access Token (classic): https://github.com/settings/tokens → generate with `repo` scope
3. In terminal:
```bash
cd /home/u/Desktop/ommi/app
git remote set-url origin https://github.com/fqhd/omnicoder.git
git push -u origin main
# When prompted: username = fqhd, password = <PAT>
```

## Option 2: SSH (if you add key)
```bash
ssh-keygen -t ed25519 -C "hdaliabadi@gmail.com" -f ~/.ssh/id_ed25519_omni -N ""
cat ~/.ssh/id_ed25519_omni.pub  # add to https://github.com/settings/keys
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519_omni
git remote set-url origin git@github.com:fqhd/omnicoder.git
git push -u origin main
```

## Option 3: Upload as release asset (easier)
Create a GitHub release manually and attach:
- `/home/u/Desktop/ommi/app/omnicoder-linux-x64` (standalone binary)
- Optionally build `.deb`/.AppImage with `npx tauri build --release` later

## Files ready to commit
- Full source (`src/`, `src-tauri/`, configs, build output `dist/`)
- `README.md` (install, usage, download link)
- `docs/` (AGENTS.md, CLAUDE.md)
- Standalone binary `omnicoder-linux-x64` (755)

Commit: c8fa0f8
