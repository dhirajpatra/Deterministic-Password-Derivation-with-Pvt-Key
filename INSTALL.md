# Install and run pwdx

pwdx comes in two forms that share the same Rust core and the same key file format:

| | Terminal (`pwdx` CLI) | App with a UI (Flutter) |
|---|---|---|
| Platforms | Linux, Windows, macOS | Linux, Windows, macOS, Android, iOS |
| Status | **Built and tested** | **Written but not yet compiled or run by the author**; expect small first-build fixes |
| Install | one binary, or build with Rust | build with Flutter + Rust |
| Key storage | one encrypted key file | OS keystore (Keychain, Android Keystore, ...) |

Start with the terminal version; it is the quickest way to check that everything works. The CLI key file and the
app's backup file are the same format, so you can use one on the other (see section 5).

There are no downloadable releases yet. You either use the Linux binary that came with the project or build from source.

---

## 1. Terminal: install

### Option A: prebuilt Linux binary (fastest)

Works on 64-bit Linux with glibc 2.34 or newer (Ubuntu 22.04+, Debian 12+, Fedora 35+, and similar).

```bash
sha256sum pwdx-linux-x86_64              # compare with the checksum you were given, if any
chmod +x pwdx-linux-x86_64
mkdir -p ~/.local/bin
mv pwdx-linux-x86_64 ~/.local/bin/pwdx
```

Make sure `~/.local/bin` is on your `PATH` (new terminals on most distributions already include it; otherwise add
`export PATH="$HOME/.local/bin:$PATH"` to `~/.bashrc` or `~/.zshrc`).

### Option B: build from source (Linux, macOS, Windows)

1. **Install Rust** (a recent stable version) from <https://rustup.rs>.
   - Windows: also install the *Build Tools for Visual Studio* with the **Desktop development with C++** workload (rustup tells you).
   - macOS: run `xcode-select --install` once.
   - Linux: you need a C compiler (`sudo apt install build-essential` on Debian/Ubuntu).
2. **Build and install** from the project folder (the one that contains `Cargo.toml`):

   ```bash
   cargo install --path crates/cli --locked
   ```

   This puts `pwdx` in `~/.cargo/bin` (Windows: `%USERPROFILE%\.cargo\bin`), which rustup already added to your `PATH`.
   Open a new terminal afterwards.

   To build without installing: `cargo build --release`, then use `target/release/pwdx` (`target\release\pwdx.exe` on Windows).

### Check the installation

```bash
pwdx selftest
```

Expected output: `selftest ok (spec v1)`. If it prints anything else, do not use that build.

Optional, if you want to run the project's own tests: `cargo test --release --workspace`.

---

## 2. Terminal: first-time setup (create your key)

You need **one key**, used on all your devices. Choose either 2a (recommended) or 2b.

### 2a. Create a new random key (recommended)

```bash
pwdx key new
```

You will be asked for a **key-file passphrase** twice (at least 12 characters; use 4+ random words). This passphrase
only protects the key file. It is different from your memorable password.

pwdx prints the **Key ID** (8 characters, not secret) and saves the encrypted key file here:

| System | Default location |
|---|---|
| Linux | `~/.config/pwdx/key.pwdxkey` (or `$XDG_CONFIG_HOME/pwdx/`) |
| macOS | `~/.config/pwdx/key.pwdxkey` |
| Windows | `%APPDATA%\pwdx\key.pwdxkey` |

To use another location: add `--key /path/to/file` to any command, or set the environment variable `PWDX_KEY_FILE`.
pwdx never overwrites an existing key file.

**Back up the key file now** (USB stick, password manager attachment, another safe place). Without it, all your derived
passwords are gone for good. Note the Key ID somewhere; it lets you check later that two devices hold the same key.

### 2b. Use your existing SSH key (`id_ed25519`)

```bash
pwdx key import-ssh ~/.ssh/id_ed25519                 # Windows PowerShell: pwdx key import-ssh $env:USERPROFILE\.ssh\id_ed25519
```

If the SSH key has a passphrase, pwdx asks for it. Then you choose a key-file passphrase as in 2a. Only the 32-byte seed
of your SSH key is copied; changing or deleting the SSH key later does not change pwdx output. A dedicated key (2a) is
the stronger choice, because an SSH public key is published.

Only unencrypted or passphrase-protected **Ed25519** OpenSSH keys work (files starting with `-----BEGIN OPENSSH PRIVATE KEY-----`).

---

## 3. Terminal: everyday use

```bash
pwdx get github.com --user me@example.com
```

pwdx asks for two things (typing is hidden):

1. `Key-file passphrase:` the one you chose in step 2.
2. `Memorable password:` your short, memorable password.

It then prints the derived password and nothing else. Use it to log in or set the password on that site.
The same inputs always give the same output, on any device that has the same key file.

Common options:

| Option | Meaning |
|---|---|
| `--user NAME` | **Required.** Username or email for the site. If the site has none, always use the same placeholder, such as `-`. |
| `--counter N` | Start at 1. Raise it to get a new password for one site (for example after a forced change). |
| `--profile full` | Letters, digits and symbols `!@#$%^&*-_=+` (default). |
| `--profile alnum` | Letters and digits only, for sites that reject symbols. |
| `--profile passphrase` | Words joined by `-`, such as `word-word-word-...`. |
| `--profile pin` | Digits only. Weak by nature; use only where nothing else is allowed. |
| `--length L` | Characters (words for `passphrase`). Defaults: 24 characters, 8 words, 6 digits. |
| `--key PATH` | Use a key file in another location. |

**Remember per site:** the exact site, username, counter, profile and length you used. Changing any of them gives a different
password. The site is simplified for you (`https://www.Login.Example.com/x` and `example.com` count as the same site).

Copy straight to the clipboard instead of showing it:

```bash
pwdx get github.com --user me@example.com | xclip -selection clipboard     # Linux (X11)
pwdx get github.com --user me@example.com | wl-copy                         # Linux (Wayland)
pwdx get github.com --user me@example.com | pbcopy                          # macOS
pwdx get github.com --user me@example.com | clip                            # Windows (cmd or PowerShell)
```

Clipboard managers may keep a copy; clear it afterwards.

Other commands: `pwdx key id` (show the Key ID), `pwdx selftest`, `pwdx help`.

Do not pass secrets on the command line or in environment variables; pwdx deliberately only accepts them at the hidden prompt.
(`--stdin` reads them from standard input for scripts and tests; do not use it for everyday work.)

---

## 4. Terminal: use the same key on another device

1. Copy the key file (`key.pwdxkey`, 107 bytes) to the other device: USB stick, secure messenger, or any file transfer.
   It is encrypted, but treat it as sensitive.
2. Install pwdx there (section 1) and put the file in that system's default location from the table above (create the folder
   if needed), or keep it anywhere and use `--key`.
3. Check: `pwdx key id` must show the **same Key ID** as on the first device.
4. Derive a password for a test site on both devices; they must match.

---

## 5. App with a UI (Flutter)

> **Honest status:** the app's Rust bridge is tested, but the Flutter code has not yet been compiled or run (the author had
> no Flutter SDK). The first build may need small fixes. The CLI above is the verified path. If something breaks, see
> `app/README.md` and tell the maintainer what the error says.

### 5.1 Prerequisites (all platforms)

- **Rust** via <https://rustup.rs> (use rustup, not a distribution package; Android and iOS builds need extra Rust targets).
- **Flutter SDK** (stable): <https://docs.flutter.dev/get-started/install>. Run `flutter doctor` and fix what it reports for
  the platform you want to build.
- **Python 3** and **bash** (the setup script uses both). On Windows, install *Git for Windows* and run the script in *Git Bash*.

Platform extras:

| Platform | Also needed |
|---|---|
| Linux | `sudo apt install clang cmake ninja-build pkg-config libgtk-3-dev libsecret-1-dev libjsoncpp-dev gnome-keyring`; a keyring service (GNOME Keyring or KWallet) must be running, because the app stores the key there |
| Windows | Visual Studio 2022 with *Desktop development with C++*; turn on *Developer Mode* in Windows settings |
| macOS | Xcode (from the App Store) and CocoaPods (`brew install cocoapods`) |
| Android | Android Studio with the SDK and the **NDK** (SDK Manager, "SDK Tools"); `flutter doctor --android-licenses` |
| iOS | a Mac with Xcode and a (free or paid) Apple developer team for signing; an iPhone or the simulator |

### 5.2 One-time setup

From the project folder:

```bash
bash scripts/bootstrap_app.sh
```

This installs the matching `flutter_rust_bridge_codegen`, creates the platform folders (Android, iOS, Linux, macOS, Windows)
around the hand-written code in `app/`, applies the security patches (no cloud backup, screenshot blocking and a protected
clipboard on Android, and so on) and generates the Dart bindings. Read the script first if you like; it only touches `app/`.

### 5.3 Run it

```bash
cd app
flutter devices                    # list what you can run on
flutter run -d linux               # or: windows, macos, <android-device-id>, <iphone-id>
```

Check the cross-platform guarantee on your machine (this runs every known-answer vector through the real Rust library):

```bash
flutter test integration_test/vectors_test.dart -d linux      # or windows / macos / a connected device
```

### 5.4 Build an installable app

| Platform | Command | Result |
|---|---|---|
| Linux | `flutter build linux --release` | `build/linux/x64/release/bundle/` (run `./pwdx_app`) |
| Windows | `flutter build windows --release` | `build\windows\x64\runner\Release\` (run `pwdx_app.exe`; keep the whole folder together) |
| macOS | `flutter build macos --release` | `build/macos/Build/Products/Release/pwdx_app.app` (drag to Applications) |
| Android | `flutter build apk --release` | `build/app/outputs/flutter-apk/app-release.apk`; install with `adb install <file>` or copy to the phone and open it (allow "install unknown apps") |
| iOS | `flutter build ios`, then open `ios/Runner.xcworkspace` in Xcode, choose your team, press Run | app on your iPhone (free accounts must re-sign every 7 days) |

The app has no network permission. On Android you can confirm with `scripts/check_android_no_network.sh app-release.apk`.

### 5.5 First start

1. The app runs a quick self-check, then asks how to get a key:
   - **Create a new key (recommended)**, or
   - **Import an OpenSSH id_ed25519 key** (pick the file; enter its passphrase if it has one), or
   - **Restore from a backup file** (a `.pwdxkey` file from the CLI or another device).
2. For a new or imported key, the app makes you **save an encrypted backup file** (choose a long backup passphrase of at least
   12 characters) and tick the box confirming you stored it safely. Keep this file; it is your recovery.
3. Tap **Finish**. The key is now stored in the system keystore of this device.

### 5.6 Everyday use

1. Open the app and unlock (fingerprint, face or device PIN where the system supports it).
2. Enter your **memorable password**, the **site** (for example `github.com`) and your **username**. Under **Options** you can
   change the counter, password type and length, and tap *Remember these settings for this site* (this saves only the
   settings, never a password).
3. Tap **Derive password**. The app shows the result hidden; tap **Show** to see it or **Copy** to copy it.
   The clipboard is cleared after 30 seconds and the result disappears from the screen after 30 seconds.
4. The memorable password field is cleared after every use. The app locks when you leave it or after 2 minutes idle.

The **key icon** opens *Key*: your Key ID, *Export encrypted backup*, *Run self-test*, and *Remove key from this device*.

### 5.7 Same key on a second device

- On the first device: **Key > Export encrypted backup**, then move the file to the new device.
- On the new device: first start > **Restore from a backup file**, pick the file, enter its passphrase.
- Check that both devices show the **same Key ID** under *Key*.

---

## 6. Check that two devices agree

Use the same key, memorable password, site, username, counter, profile and length on both. For example in the terminal on one
device and in the app on another: the derived password must be identical. If it is not, see the next section.

## 7. Troubleshooting

| Symptom | Likely cause and fix |
|---|---|
| `command not found: pwdx` | Folder with the binary is not on `PATH`. Open a new terminal, or use the full path. |
| `selftest` fails | Broken build. Rebuild with a current stable Rust (`rustup update`); do not use the binary. |
| `error: decryption failed` | Wrong key-file passphrase, or the file is damaged. Retype it carefully; try your backup copy. |
| `cannot read key file` | Wrong path. Run `pwdx key id --key /full/path/key.pwdxkey` or set `PWDX_KEY_FILE`. |
| `cannot create ... (it will not be overwritten)` | A key file already exists there. Use `--key` with a new path, or move the old file away on purpose (keep a backup!). |
| `key stretching failed` / not enough memory | Each derivation needs about 64 MiB of free memory. Close other programs. pwdx never lowers this setting, because that would change your passwords. |
| `length is out of range` | Allowed lengths: 8-128 characters (`full`, `alnum`), 4-16 words (`passphrase`), 4-12 digits (`pin`). |
| Passwords differ between devices | Check, in this order: same **Key ID**; memorable password typed exactly the same (spaces, capitals; accents are fine); same username (capitals matter); same counter, profile and length; same site (`github.com` vs a different domain). |
| Linux app cannot save the key | No keyring running. Start GNOME Keyring or KWallet (log in through a normal desktop session). |
| App refuses to derive and shows a self-test message | The build is broken; do not use it. Rebuild. |
| Lost the key file and have no backup | The passwords cannot be recovered by anyone. Create a new key and change each site's password. |

Changing your memorable password or your key changes **every** derived password, so you would have to update every site.

## 8. Update and uninstall

- **Update CLI:** get the new source, then run `cargo install --path crates/cli --locked --force`. Spec v1 outputs never change
  within v1, so your passwords stay the same.
- **Uninstall CLI:** `cargo uninstall pwdx` (or delete the binary). Your key file stays; delete it only if you have a backup and
  really want it gone (`~/.config/pwdx/` or `%APPDATA%\pwdx\`).
- **Uninstall the app:** remove it like any app; use *Key > Remove key from this device* first if you want the key erased
  from that device's keystore. Your backup file is not affected.
