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

### 2.5 Configuration

The build reads `config/api.toml` (committed), `config/user.toml` and
`config/secrets.toml`. Create the last two from the templates:

```sh
cp config/user.example.toml config/user.toml        # location, topics, instruments
cp config/secrets.example.toml config/secrets.toml  # Wi-Fi and API keys
```

The build validates all three, prints the derived API schedule, and stops with
an explanation if anything is missing, still a placeholder, or inconsistent.
To build without personal files (CI, or just to try the display), set
`DT_ALLOW_EXAMPLE_CONFIG=1`; the firmware then has no working Wi-Fi or keys.

### 2.6 Build and flash

```sh
cd firmware
cargo run --release     # builds, flashes with espflash and opens the serial monitor
```

## 3. Working on the screens

Everything that draws runs on the PC. To see a screen:

```sh
cargo run -p dt-sim -- weather              # writes weather-full.png
cargo run -p dt-sim -- news failed news.png # states: full, loading, failed, stale
```

Every page in every state, the icons, charts and type specimen are golden
images in `crates/dt-sim/golden/`, compared pixel for pixel by `cargo test`.
When a test fails, the frame it produced is written to
`target/golden-failures/<name>.actual.png` (CI uploads that folder as the
`golden-failures` artifact). If the change is intended:

```sh
UPDATE_GOLDEN=1 cargo test -p dt-sim
git diff --stat crates/dt-sim/golden   # then look at each changed image
```

Commit the updated images with the change that caused them.

## 4. Versions

| Item | Version | Where it is pinned |
|---|---|---|
| Host Rust | stable, 1.82 or newer | `rust-version` in `Cargo.toml` |
| Firmware Rust | `esp` channel | `firmware/rust-toolchain.toml` |
| ESP-IDF | v5.5.3 | `firmware/.cargo/config.toml` |
