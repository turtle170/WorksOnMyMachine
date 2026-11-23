# womm

**Enterprise-grade declarative executable synthesis.**

womm compiles project plans into native executables. Write your plan in any format your organisation already uses: Markdown, plain text, or any other UTF-8 content. womm then runs it through a rigorous, fully verified, three-stage toolchain.[^1]

| Capability | womm |
|---|---|
| Accepts any Unicode project plan | Yes |
| SHA-512 + RSA-2048 signed builds | Yes |
| Tunable parser optimisation (`-O0` to `-O3`) | Yes[^2] |
| Human-readable build output | Yes, it's Markdown |
| Native executables for Windows and macOS | Yes[^3] |
| Works on your machine | See [Limitations](#limitations) |

## Installation

```bash
cargo install --path .
```

## Usage

### 1. Author

Save your plan as `<name>.womm`:

```markdown
# Synergy Platform

- Align stakeholders
- Rewrite it in Rust
- Ship it (on my machine)
```

### 2. Parse

```bash
womm parse synergy.womm -O2
```

The parser hashes your plan and emits `.kludge` intermediate objects into `synergy.kludged/`, next to the source. Like `.o` files, but more of them.

| Level | Kludge objects |
|---|---|
| `-O0` | 4 |
| `-O1` | 16 |
| `-O2` | 64 |
| `-O3` | 256 |

### 3. Build

```bash
womm build synergy.kludged
```

The compiler verifies every kludge object, hashes them with **SHA-512**, and signs the digest with your machine's **RSA-2048** key. The result is a 256-byte signature, written as exactly 512 hexadecimal digits into `synergy.womm.md` along with the build parameters and the public key.

Your machine key is generated on first build and kept in your config directory. The same plan built on a different machine produces a different signature.[^5]

### 4. Link

```bash
womm parsy-builder synergy.womm.md
```

The linker parses the 512 signature digits with great care:

```
   Lexed 512 hexadecimal digits across 8 line(s)
   Verified RSA-2048 signature over SHA-512 digest (key 44c4f1817ecc8069)
   Paired 512 nibbles into 256 instructions
   Instruction mix: ADDx17 COLORx15 ENDx14 EXCUSEx20 GLYPHRx17 ...
   Control flow: loop depth 1, 0 unterminated loop(s), 6 stray END(s), HALT at 0x00c0
```

Each byte of the signature is one instruction for the **Kludge Virtual Machine**: the high nibble is the opcode, the low nibble its argument. Every signature is a valid program and every program terminates. The linker produces `synergy` (`synergy.exe` on Windows).

### 5. Run

```
$ ./synergy
 bandwidth ░░░░░░
 roadmap   ▪ agile □□□□       stakeholder □□□□□□
░▼░▼░▼▌ paradigm ▐▼▼▼▼▼▼▼▼◆
  // known issue, wontfix
```

The output is fully determined by your plan, your parser optimisation level, and your machine. It is not otherwise determined by anything.

Built executables also accept `--womm-disasm` (instruction listing) and `--womm-signature` (the 512 digits).

#### Kludge VM instruction set

| Op | Mnemonic | Effect |
|---|---|---|
| `0` | GLYPH | Print glyph *a* from the current set |
| `1` | REPEAT | Repeat the last glyph *a*+1 times |
| `2` | COLOR | Set terminal colour *a* |
| `3` | NEWLINE | New line (reset colour if *a* ≥ 8) |
| `4` | GLYPHSET | Switch glyph set |
| `5` | LOOP | Begin a loop of 2 to 4 iterations |
| `6` | END | End loop |
| `7` | WORD | Print a business term |
| `8` | SLEEP | Pause for dramatic effect |
| `9` | ADD | Increment the register |
| `A` | GLYPHR | Print a glyph chosen by the register |
| `B` | SPACE | Print spaces |
| `C` | STRIPE | Print a glyph eight times |
| `D` | EXCUSE | Print an excuse |
| `E` | SKIP | Skip up to two instructions |
| `F` | MIRROR | Print a mirrored glyph (`FF` halts) |

### Getting help

```bash
womm --help
```

Help is available.[^4]

## Limitations

- Executables are built for the platform that ran `parsy-builder`.
- The compiler embeds itself in every executable. This is a design decision.
- See the [fine print](#fine-print).

---

[^1]: The three stages are parse, build, and parsy-builder. The middle stage outputs a Markdown file. This is also a design decision.
[^2]: Optimisation levels apply to the parser, not the compiler. They make more files. That is all they do.
[^3]: One at a time, on the platform you build on.
[^4]: Eventually.
[^5]: It works on my machine.

## License

MIT. See [LICENSE](LICENSE).

## Fine print

<sub>womm is pathetic, self-aware slopware. It turns a text file into a large number of text files, then signs them with enterprise-grade cryptography, then treats the signature as source code. Your plan is discarded along the way. What you get is a terminal screensaver that apologises. It worked on my machine. If it doesn't work on yours, there are a few unlisted commands that clean up after it. You may find them by expressing how you feel.</sub>
