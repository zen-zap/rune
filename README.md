# Rune

A custom Unix-like shell implemented in Rust.

---

## Features

### Command Execution
- **External Commands:** Runs any executable available in `$PATH` (e.g., `ls`, `grep`, `cat`).
- **Built-ins:** Supports standard built-in commands:
  - `cd [dir]` — Change directory
  - `pwd` — Print current working directory
  - `exit [code]` — Exit the shell with an optional status code

### Pipelines
- **Pipeline Processing:** Supports chaining commands with `|`, e.g.:
  ```sh
  ls | grep src | wc
  ```
  Each stage runs in its own process, with standard input/output properly connected via pipes.

### Miscellaneous
- **Process Groups:** Each pipeline sets up a process group for future job control features (`setpgid`), laying the groundwork for `Ctrl+Z`, backgrounding, and job management.
- **Proper FD Handling:** Pipes, stdin/stdout redirection, and close-on-exec all handled robustly to avoid resource leaks and deadlocks.


## Build and Run Instructions

### 1. **Prerequisites**
- [Rust](https://www.rust-lang.org/tools/install) (latest stable recommended)
- Unix-like OS (Linux, macOS, WSL, etc.)

### 2. **Clone the Repository**
```sh
git clone https://github.com/zen-zap/rune.git
cd rune
```

### 3. **Build**
```sh
cargo build
```
Or, for a development build with debug info:
```sh
cargo build --release
```

### 4. **Run**
```sh
cargo run
```
Or, run the compiled binary directly:
```sh
./target/debug/rune
# Or, for the release build:
./target/release/rune
```

### 5. **Usage Example**
```
RuneShell $ ls | grep src | wc
      1       1       4
RuneShell $ cd ..
RuneShell $ pwd
/home/username
RuneShell $ exit
```

---

## Important!

It looks for a rune.conf file in `../rune` which is the parent directory of the rune directory, when you clone the repository.

Example Configuration:
```
/bin
/usr/bin
/usr/local/bin
/sbin
/usr/sbin
/home/username/.local/bin
./bin
```

Any contributions are welcome!
