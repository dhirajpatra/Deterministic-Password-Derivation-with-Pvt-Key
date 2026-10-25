//! pwdx CLI. Secrets are read from a no-echo terminal prompt (or from stdin with `--stdin`, for automation).
//! They are never accepted as arguments or environment variables.

use pwdx_core::{
    derive, export_backup, import_backup, import_openssh, KeySeed, Profile, SiteRequest,
};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use zeroize::Zeroizing;

const HELP: &str = "\
pwdx - stateless, offline password derivation (spec v1)

USAGE:
  pwdx key new                    create a new random key, stored encrypted
  pwdx key import-ssh <FILE>      import an OpenSSH id_ed25519 (plain or passphrase-protected)
  pwdx key id                     show the key's non-secret Key ID
  pwdx get <SITE> --user <USER> [--counter N] [--profile full|alnum|passphrase|pin] [--length L]
  pwdx selftest                   run the built-in known-answer check

OPTIONS:
  --key <PATH>    key file (default: ~/.config/pwdx/key.pwdxkey, or $PWDX_KEY_FILE)
  --stdin         read secrets as lines from stdin instead of a terminal prompt (automation/tests)

The key file is encrypted with a passphrase (Argon2id + XChaCha20-Poly1305) and is also your backup:
copy it to other devices. Keep a copy somewhere safe: without it every derived password is lost.
";

struct Opts {
    key_path: PathBuf,
    stdin: bool,
    user: Option<String>,
    counter: u32,
    profile: Profile,
    length: Option<usize>,
    positional: Vec<String>,
}

fn default_key_path() -> PathBuf {
    if let Some(p) = std::env::var_os("PWDX_KEY_FILE") {
        return PathBuf::from(p);
    }
    if let Some(d) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(d).join("pwdx").join("key.pwdxkey");
    }
    if let Some(d) = std::env::var_os("APPDATA") {
        return PathBuf::from(d).join("pwdx").join("key.pwdxkey");
    }
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h)
            .join(".config")
            .join("pwdx")
            .join("key.pwdxkey");
    }
    PathBuf::from("key.pwdxkey")
}

fn parse(args: Vec<String>) -> Result<Opts, String> {
    let mut o = Opts {
        key_path: default_key_path(),
        stdin: false,
        user: None,
        counter: 1,
        profile: Profile::Full,
        length: None,
        positional: vec![],
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut val = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "--key" => o.key_path = PathBuf::from(val("--key")?),
            "--stdin" => o.stdin = true,
            "--user" => o.user = Some(val("--user")?),
            "--counter" => {
                o.counter = val("--counter")?
                    .parse()
                    .map_err(|_| "--counter must be a number")?
            }
            "--profile" => {
                o.profile = Profile::parse(&val("--profile")?)
                    .ok_or("unknown profile (full, alnum, passphrase, pin)")?
            }
            "--length" => {
                o.length = Some(
                    val("--length")?
                        .parse()
                        .map_err(|_| "--length must be a number")?,
                )
            }
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => o.positional.push(a),
        }
    }
    Ok(o)
}

fn prompt(label: &str, stdin: bool) -> Result<Zeroizing<String>, String> {
    if stdin {
        eprint!("{label}: ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        let t = line.trim_end_matches(['\n', '\r']).to_string();
        line.clear();
        Ok(Zeroizing::new(t))
    } else {
        rpassword::prompt_password(format!("{label}: "))
            .map(Zeroizing::new)
            .map_err(|e| format!("cannot read from terminal: {e}"))
    }
}

fn new_passphrase(stdin: bool) -> Result<Zeroizing<String>, String> {
    eprintln!("Choose a key-file passphrase. It protects the key file; make it long and unique (4+ random words).");
    let a = prompt("Key-file passphrase", stdin)?;
    let b = prompt("Repeat passphrase", stdin)?;
    if *a != *b {
        return Err("passphrases do not match".into());
    }
    if a.chars().count() < 12 {
        return Err("passphrase is too short (use at least 12 characters)".into());
    }
    Ok(a)
}

fn write_key_file(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path).map_err(|e| {
        format!(
            "cannot create {} (it will not be overwritten): {e}",
            path.display()
        )
    })?;
    f.write_all(bytes).map_err(|e| e.to_string())
}

fn load_key(path: &PathBuf, stdin: bool) -> Result<KeySeed, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("cannot read key file {}: {e}", path.display()))?;
    let pass = prompt("Key-file passphrase", stdin)?;
    import_backup(&bytes, &pass).map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "help" || args[0] == "--help" || args[0] == "-h" {
        print!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "selftest" => {
            pwdx_core::self_test().map_err(|e| e.to_string())?;
            println!("selftest ok (spec {})", pwdx_core::SPEC_VERSION);
            Ok(())
        }
        "key" => {
            if args.is_empty() {
                return Err("usage: pwdx key new | import-ssh <FILE> | id".into());
            }
            let sub = args.remove(0);
            let o = parse(args)?;
            match sub.as_str() {
                "new" => {
                    let pass = new_passphrase(o.stdin)?;
                    let key = KeySeed::generate().map_err(|e| e.to_string())?;
                    let file = export_backup(&key, &pass).map_err(|e| e.to_string())?;
                    write_key_file(&o.key_path, &file)?;
                    eprintln!("Key created: {}", o.key_path.display());
                    eprintln!(
                        "Key ID: {}   (same ID on every device = same key)",
                        key.key_id()
                    );
                    eprintln!("IMPORTANT: back this file up now. Without it, every derived password is lost.");
                    Ok(())
                }
                "import-ssh" => {
                    let src = o
                        .positional
                        .first()
                        .ok_or("usage: pwdx key import-ssh <FILE>")?;
                    let data = std::fs::read(src).map_err(|e| format!("cannot read {src}: {e}"))?;
                    let key = match import_openssh(&data, None) {
                        Ok(k) => k,
                        Err(pwdx_core::Error::PassphraseRequired) => {
                            let p = prompt("SSH key passphrase", o.stdin)?;
                            import_openssh(&data, Some(p.as_bytes())).map_err(|e| e.to_string())?
                        }
                        Err(e) => return Err(e.to_string()),
                    };
                    eprintln!("Note: an SSH public key is published; a dedicated key (`pwdx key new`) is the stronger choice.");
                    eprintln!("Note: only the seed is copied; rotating your SSH key later will not change pwdx output.");
                    let pass = new_passphrase(o.stdin)?;
                    let file = export_backup(&key, &pass).map_err(|e| e.to_string())?;
                    write_key_file(&o.key_path, &file)?;
                    eprintln!("Key imported: {}", o.key_path.display());
                    eprintln!("Key ID: {}", key.key_id());
                    eprintln!("IMPORTANT: back this file up now.");
                    Ok(())
                }
                "id" => {
                    let key = load_key(&o.key_path, o.stdin)?;
                    println!("{}", key.key_id());
                    Ok(())
                }
                _ => Err(format!("unknown key command {sub}")),
            }
        }
        "get" => {
            let o = parse(args)?;
            let site = o
                .positional
                .first()
                .ok_or("usage: pwdx get <SITE> --user <USER>")?
                .clone();
            let user = o.user.clone().ok_or(
                "--user is required (use a fixed placeholder such as '-' if the site has none)",
            )?;
            pwdx_core::self_test().map_err(|e| format!("{e}; refusing to derive"))?;
            let key = load_key(&o.key_path, o.stdin)?;
            let secret = prompt("Memorable password", o.stdin)?;
            let req = SiteRequest {
                site: &site,
                user: &user,
                counter: o.counter,
                profile: o.profile,
                length: o.length.unwrap_or_else(|| o.profile.default_length()),
            };
            let pw = derive(&key, &secret, &req).map_err(|e| e.to_string())?;
            if o.profile == Profile::Pin {
                eprintln!("Warning: a PIN has very little entropy; use it only where nothing else is allowed.");
            }
            println!("{}", *pw);
            Ok(())
        }
        _ => Err(format!("unknown command {cmd}; try `pwdx help`")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("error: {m}");
            ExitCode::FAILURE
        }
    }
}
