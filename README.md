# CTF Tools

CTF Tools is a desktop toolbox for CTF practice, competition workflows, and local security lab work. The project uses Rust for the core execution path and performance-sensitive operations, while Python workers provide a flexible extension layer for script-friendly tasks. The goal is to keep common CTF work fast, local, and organized across codecs, Web, Crypto, forensics, Pwn, reverse engineering, and challenge notes.

The desktop app uses English by default and includes a language setting for Chinese. Core functionality is exposed through a shared Operation registry, so the CLI and GUI use the same runner instead of maintaining separate implementations.

## Features

- **Recipe-based operation chains**: Search tools in the Operations workspace, add them to a Recipe, reorder steps, and run multi-step encode/decode/transform workflows.
- **Codecs and transforms**: Base64, Base32, Base45, Base58, Base62, Base85, URL, HTML, Unicode, ASCII, Hex, binary, octal, decimal, hexdump, endian swap, Morse, Brainfuck, and auto decode.
- **Hashes and crypto helpers**: MD5, SHA1, SHA2, SHA3, NTLM, SM3, RC4, repeating-key XOR, ROT13, ROT47, Atbash, Vigenere, Caesar brute force, and single-byte XOR brute force.
- **Web / HTTP / JWT**: Raw HTTP request parsing, Python `requests` / `httpx` code generation, curl and fetch generation, JWT decode, HS256 sign/verify, and weak-key checks.
- **Asset classification**: Extract and group URLs, domains, IP addresses, CIDR-like ranges, emails, phone numbers, and ID-card-like values from mixed text.
- **Files, stego, and traffic analysis**: Hex viewer, entropy analysis, image Data URI generation, GIF frame extraction, PCAP HTTP/DNS/ICMP/TCP summaries, and USB HID key parsing.
- **Pwn and reverse engineering**: ELF/PE/Mach-O metadata, strings extraction, cyclic patterns, pack/unpack helpers, and x86_64 shellcode asm/disasm.
- **Local tool launcher**: Manage Python scripts, Java tools, shell commands, GUI apps, and URLs with favorites, recent items, environment scanning, and path copying.
- **Challenge workspace**: Entry points for challenge notes and writeup templates.

## Desktop App

Start the GUI:

```bash
cargo run -p ctf-app
```

The Operations workspace is designed for high-frequency solving work:

1. Find a tool from the category panel or search box.
2. Double-click it or add it to the Recipe.
3. Provide text, bytes, or a file path as input.
4. Run the Recipe and inspect the result, trace, and warnings.

The Launcher workspace manages external tools:

1. Add a local script, JAR, binary, GUI app, or URL.
2. Configure default Python and Java environments.
3. Use favorites, recent items, and search to launch tools quickly.

## CLI

List operations:

```bash
cargo run -p ctf-cli -- list
```

Search operations:

```bash
cargo run -p ctf-cli -- search base64
```

Run an operation:

```bash
cargo run -p ctf-cli -- run base64.decode --text "ZmxhZ3t0ZXN0fQ=="
```

Keyed crypto operations such as RC4 and repeating-key XOR accept inline options:

```bash
cargo run -p ctf-cli -- run rc4.apply --text $'key=Key\ninput=hex\noutput=text\n\nbbf316e8d940af0ad3'
```

Launcher commands:

```bash
cargo run -p ctf-cli -- launcher list
cargo run -p ctf-cli -- launcher search cyber
cargo run -p ctf-cli -- launcher scan-envs
```

## Project Layout

```text
crates/
  ctf-app       Desktop GUI based on eframe/egui
  ctf-cli       Command-line entry point
  ctf-core      Operation schema, registry, and runner types
  ctf-runner    Rust handlers and Python worker dispatch
  ctf-codecs    Codecs, transforms, and auto decode
  ctf-crypto    Hashes, classical ciphers, and password helpers
  ctf-web       HTTP, JWT, and asset classification
  ctf-stego     File, image, PCAP, and USB HID analysis
  ctf-pwn       Pwn and reverse engineering helpers
  ctf-launcher  Local tool launcher data model and environment management
registry/
  operations.toml  Built-in Operation registry
python/
  Python worker and tests
docs/
  Development docs, plugin model, and staged plans
```

## Safety Boundaries

- Default features focus on local parsing, conversion, and analysis.
- Active scanning, request replay, and external intelligence API queries are disabled by default.
- HTTP code generation redacts sensitive headers and cookies by default.
- Long-running tasks are constrained by the shared runner with timeout and output-size limits.
- Python workers use a JSON Lines protocol for isolation, recovery, and plugin-oriented extension.

## Development And Testing

Common gates:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
PYTHONPATH=python .venv/bin/python -m pytest python/tests
```

Documentation:

- [Development Guide](./docs/DEVELOPMENT.md)
- [Plugin And Adapter Model](./docs/PLUGIN_MODEL.md)
- [Staged Plan](./docs/MVP_PLAN.md)
