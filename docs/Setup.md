# Development setup

This project has two halves with different toolchains:

| Part | Directory | Toolchain | Needs the board |
|---|---|---|---|
| Host crates: config, core, UI, simulator | repository root | stable Rust | No |
| Firmware: C HAL, `dt-hal`, `dt-firmware` | `firmware/` | Espressif `esp` Rust + ESP-IDF | Yes, to flash |

Everything below assumes Linux. macOS works the same way; on Windows use WSL.

## 1. Host toolchain

Install Rust with [rustup](https://rustup.rs), then, from the repository root:

```sh
cargo test
```

That is all the host crates need. The root `Cargo.toml` excludes `firmware/`.

## 2. Firmware toolchain

### 2.1 System packages

ESP-IDF needs git, Python 3 with `venv`, CMake, Ninja, and libusb/libudev for
flashing. On Arch:

```sh
sudo pacman -S --needed git python cmake ninja libusb systemd-libs clang
```

On Debian/Ubuntu:

```sh
sudo apt install git python3 python3-venv cmake ninja-build libusb-1.0-0 libudev-dev clang
```

### 2.2 Rust tools

Installing these needs stable Rust 1.88 or newer (`rustup update stable`).

```sh
cargo install espup ldproxy espflash --locked
espup install          # installs the `esp` Rust toolchain and the Xtensa GCC/LLVM
```

`espup install` writes `~/export-esp.sh`. Load it in every shell that builds
the firmware (or add it to your shell profile):

```sh
. ~/export-esp.sh
```

`firmware/rust-toolchain.toml` selects the `esp` channel automatically inside
`firmware/`.

### 2.3 ESP-IDF

Nothing to install by hand. On the first firmware build, `esp-idf-sys`
downloads the ESP-IDF version pinned in `firmware/.cargo/config.toml`
(`ESP_IDF_VERSION`) and its tools into `firmware/.embuild/`. The first build
takes several minutes and about 2 GB of disk.

### 2.4 Serial port access

The driver board's USB bridge (CH343P) appears as `/dev/ttyACM0`. Your user
needs permission to open it:

```sh
sudo usermod -aG uucp $USER     # Arch
sudo usermod -aG dialout $USER  # Debian/Ubuntu
```

Log out and back in for the group change to apply.

On the board, DIP switch SW1 position 1 ("USB to UART") must be ON to flash.

### 2.5 Build and flash

```sh
cd firmware
cargo run --release     # builds, flashes with espflash and opens the serial monitor
```

## 3. Versions

| Item | Version | Where it is pinned |
|---|---|---|
| Host Rust | stable, 1.82 or newer | `rust-version` in `Cargo.toml` |
| Firmware Rust | `esp` channel | `firmware/rust-toolchain.toml` |
| ESP-IDF | v5.5.3 | `firmware/.cargo/config.toml` |
