# womm

**Enterprise-grade declarative executable synthesis. Now neurally assisted.**

womm compiles project plans into native executables. Write your plan in any format your organisation already uses: Markdown, plain text, or any other UTF-8 content. womm then runs it through a rigorous, fully verified, AI-powered toolchain.[^1]

| Capability | womm |
|---|---|
| Accepts any Unicode project plan | Yes |
| SHA-512 + RSA-2048 signed builds | Yes, five times per build |
| On-device neural network (NoNet) | Yes, 833 parameters[^6] |
| Tunable parser optimisation (`-O0` to `-O3`) | Yes[^2] |
| XLA compute graph (StableHLO) intermediate | Yes |
| Human-readable build output | Yes, it's Markdown |
| Native executables for Windows and macOS | Yes[^3] |
| Static analysis and code review | Yes, by the executable, of itself |
| Works on your machine | See [Limitations](#limitations) |

## Installation

Each [release](https://github.com/turtle170/WorksOnMyMachine/releases) provides:

| Platform | Installer | Installs to |
|---|---|---|
| macOS | `womm-<version>-<arch>.pkg` | `~/Documents/Womm` |
| Windows | `womm-<version>-x86_64-pc-windows-msvc-setup.exe` | `%AppData%\Womm` |
| Any | `.tar.gz` / `.zip` portable archive | wherever you put it |

The installers include Python.[^7] The installers are not code-signed, so macOS and Windows will warn you about them. They are correct to.

From source:

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

| Level | Kludge objects | Optimisation passes |
|---|---|---|
| `-O0` | 4 | none |
| `-O1` | 16 | 1 |
| `-O2` | 64 | 2 |
| `-O3` | 256 | 3, and a note of concern |

The parser also reviews your plan and emits warnings. Each warning cites a real line and quotes a real word from it:

```
warning[Wb3bd]: line 4: the word "rewrite" assumes 6 feature flags; this is fine
womm: 1 warning(s) emitted, 0 heeded
```

Pass `-W no` to suppress warnings. They remain true.

### 3. Build

```bash
womm build synergy.kludged
```

The build is where the engineering happens:

1. **NoNet is constructed.** It is an 8 → 32 → 16 → 1 neural network. It does nothing yet.
2. **NoNet is trained.** AdamW tunes it on the compiler binary itself.
3. Every kludge object is verified and hashed with **SHA-512**. The digest is signed with your machine's **RSA-2048** key.
4. **The hash chain.** The signature is split into 4 new kludge objects. Each one is hashed and signed again, and the 4 signatures are joined into a 1,024-byte **big hash**.
5. NoNet reads the big hash and appends an **extra hash** of its own. Its length is up to NoNet.
6. The result is split into yet more kludge objects, parsed, and lowered to an **XLA compute graph in StableHLO**.
7. Everything is written to `synergy.womm.md`.
8. The **garbage collector** runs.

The build re-analyses your plan with the signature and warns again. Warnings that agree with the parser are marked `(still)`.

Your machine key is generated on first use and kept in your config directory. The same plan built on a different machine produces a different program.[^5]

### 4. Link

```bash
womm parsy-builder synergy.womm.md
```

The linker reads the manifest with great care:

```
   Lexed 2658 hexadecimal digits across 3 sections
   Verified 5 RSA-2048 signatures (key 2fbd6ec49c5e0000)
   Paired 2146 nibbles into 1073 instructions
   Instruction mix: ADDx62 COLORx61 ENDx66 EXCUSEx50 ...
   Control flow: loop depth 4, 14 unterminated loop(s), 6 stray END(s), HALT at 0x015f
   Personality: Smug (this was not a choice)
```

Each byte of the final hash is one instruction for the **Kludge Virtual Machine**: the high nibble is the opcode, the low nibble its argument. Every hash is a valid program and every program terminates. Longer programs are allowed to print more. The linker produces `synergy` (`synergy.exe` on Windows).

### 5. Run

```
$ ./synergy
synergy -- built with womm 0.2.0 from 64 kludge object(s) at -O2
machine key 2fbd6ec49c5e0000, 1073 instructions, mood: Smug

▒▪▪▪▪▪▪▪▪▪▀▒▪▌◆◆◆◆◆◆◆◆ ping ▌ alignment  roadmap
  // probably your PATH

Program complete. Works on MY machine.

── review ────────────────────────────────────────
reviewer: self (the best reviewer available) (conflict of interest: yes)
  0x0067  blocking: 6 stray END(s); who are these ENDs for; consider a feature flag
  0x015f  question: HALT here; 721 instruction(s) after it never run; please add a test (we both know you won't)
verdict: REQUEST CHANGES (will merge anyway)
```

Every executable has a **mood**, chosen by its first instruction: Anxious, Corporate, Burnt-out, Smug, Haunted, Intern, Philosophical or Retired. When it runs on a machine other than the one it was built on, it notices.

Built executables also accept:

| Flag | Effect |
|---|---|
| `--womm-disasm` | Instruction listing |
| `--womm-signature` | The final hash |
| `--womm-mood` | The mood, and a horoscope |
| `--womm-no-review` | Skip the self-review |

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

### Visualise

```bash
womm visualise synergy.womm.md
```

womm visualises the manifest:

```
👁
```

It also translates the StableHLO graph into `synergy.nonet.py`, a readable, dependency-free Python program. Running that file reproduces NoNet's extra hash exactly, bit for bit. This is the most useful thing womm does.

### Blame

```bash
womm blame synergy.womm
```

Shows who wrote each line of a plan.

### Packages

womm ships with a package store. Packages are installed with:

```bash
womm ship <name>
```

The package store is in the folder you installed womm to.[^8] Installing to PAHTH is recommended. It gives extra functionality and accessibility.

### Getting help

```bash
womm --help
```

Help is available.[^4]

## Limitations

- Executables are built for the platform that ran `parsy-builder`.
- The compiler embeds itself in every executable. This is a design decision.
- NoNet is trained on the compiler binary, so a different build of womm produces different programs.
- See the [fine print](#fine-print).

---

[^1]: The stages are parse, build and parsy-builder. The middle stage outputs a Markdown file. This is also a design decision.
[^2]: Optimisation levels apply to the parser, not the compiler. They make more files. That is all they do.
[^3]: One at a time, on the platform you build on.
[^4]: Eventually.
[^5]: It works on my machine.
[^6]: 288 + 528 + 17. 3.33 KB as float32, or 833 bytes if you are generous with quantisation. Version 0.1.0 had none, and was codenamed "Works On Tuesday". This is 0.2.0, "NoNet v0".
[^7]: The newest standalone CPython at release time, for `womm visualise --run`. The portable archives do not include Python. `visualise` still writes the file.
[^8]: There is no command that lists the packages. Look inside `packages.tar`.

## License

MIT. See [LICENSE](LICENSE).

## Fine print

<sub>womm is pathetic, self-aware slopware. It turns a text file into a large number of text files, signs them, turns the signature into more text files, signs those, asks a neural network it trained on itself to add some more, and then treats all of that as source code. Your plan is discarded along the way. What you get is a terminal screensaver that apologises and then reviews itself. NoNet has been trained to say no, and there are commands for talking to it, training it and making it fight, but none of them are listed here, because none of them were approved. It worked on my machine. If it doesn't work on yours, there are a few unlisted commands that clean up after it. You may find them by expressing how you feel.</sub>
