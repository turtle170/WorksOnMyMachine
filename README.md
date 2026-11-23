# womm

**Enterprise-grade declarative executable synthesis.**

womm compiles project plans into native executables. Write your plan in any format your organisation already uses: Markdown, plain text, or any other UTF-8 content. womm then runs it through a rigorous, fully verified, three-stage toolchain.[^1]

| Capability | womm |
|---|---|
| Accepts any Unicode project plan | Yes |
| SHA-256 integrity at every stage | Yes |
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

The compiler verifies every kludge object, reassembles the plan, and emits `synergy.womm.md`: a build manifest containing the exact build parameters and the compiled binary.

### 4. Link

```bash
womm parsy-builder synergy.womm.md
```

Produces `synergy` (`synergy.exe` on Windows). Run it to execute your project plan:

```
[1/4] # Synergy Platform ... done
[2/4] - Align stakeholders ... done
[3/4] - Rewrite it in Rust ... done
[4/4] - Ship it (on my machine) ... done

Project complete. Works on my machine.
```

Pass `--womm-source` to a built executable to recover the original plan.

### Getting help

```bash
womm --help
```

Help is available.[^4]

## Limitations

- Executables are built for the machine that ran `womm build`. Manifests from another platform are rejected by `parsy-builder`.
- The compiler embeds itself in every executable. This is a design decision.
- See the [fine print](#fine-print).

---

[^1]: The three stages are parse, build, and parsy-builder. The middle stage outputs a Markdown file. This is also a design decision.
[^2]: Optimisation levels apply to the parser, not the compiler. They make more files. That is all they do.
[^3]: One at a time, on the platform you build on.
[^4]: Eventually.

## License

MIT. See [LICENSE](LICENSE).

## Fine print

<sub>womm is pathetic, self-aware slopware. It turns a text file into a large number of text files, then into a Markdown file containing a base64 copy of itself, and then into an executable that reads your to-do list back to you, slowly. It worked on my machine. If it doesn't work on yours, there are a few unlisted commands that clean up after it. You may find them by expressing how you feel.</sub>
