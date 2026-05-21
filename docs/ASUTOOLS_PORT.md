# asuTools Port Notes

Source reference: https://github.com/lsdogXG/asutools

License: MIT. The launcher module is a Rust implementation of the same local-tool-launcher feature set and keeps the JSON schema compatible with asuTools data files.

Implemented feature map:

- Tool types: `python`, `java`, `shell`, `gui`, `url`
- Data files: `tools.json`, `categories.json`, `environments.json`, `settings.json`
- Atomic JSON writes under `~/Library/Application Support/CTF Tools/launcher/`
- Tool fields: id, name, type, path, args, category, env_id, tags, description, favorite, last_used
- Environment fields: id, name, type, path, version, source, tags, javafx
- Environment scanning: system/brew Python, venv roots, conda roots, Java/JDK roots, `$JAVA_HOME`, bundled Workspace JDKs
- Launch behavior: URL/path open, Terminal dispatch for Python/Java/shell, default environment fallback
- GUI behavior: launcher workspace, categories, favorites, recent tools, search, add/edit/remove, favorite toggle, copy path, rescan environments, manual environment add, default environment selection
- Settings behavior: dark/light theme persistence, language-aware settings panel, asuTools data import, TH_Tools bootstrap, JavaFX JAR environment binding
- Keyboard behavior: search focus, new/edit/favorite shortcuts, Enter launch, Escape clear search, Backspace remove, category switching, macOS window shortcuts
- CLI behavior: `ctf-tools launcher list/search/scan-envs/import-asutools/bootstrap-th/bind-javafx`
- Packaging behavior: macOS `.app` and `.dmg` scripts under `scripts/`

Deliberate integration differences:

- The UI is implemented in egui to match the CTF Tools desktop app instead of embedding PyQt6.
- The data directory is namespaced under CTF Tools rather than writing into asuTools' own directory.
- Theme data is stored with the asuTools-compatible launcher settings schema and applied to the CTF Tools UI layer.
