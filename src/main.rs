// We need this feature as it changes `dylib` linking behavior and allows us to link to
// `rustc_driver`.
#![feature(rustc_private)]
// warn on lints, that are included in `rust-lang/rust`s bootstrap
#![warn(rust_2018_idioms, unused_lifetimes)]

use std::env;
use std::path::PathBuf;
use std::process::{self, Command};

use anstream::println;
use soloud::Wav;
use tiny_rng::Rand;
use soloud::*;

#[allow(clippy::ignored_unit_patterns)]
fn show_help() {
    println!("{}", help_message());
}

#[allow(clippy::ignored_unit_patterns)]
fn show_version() {
    let version_info = rustc_tools_util::get_version_info!();
    println!("{version_info}");
}

pub fn main() {
    // Check for version and help flags even when invoked as 'cargo-clippy'
    if env::args().any(|a| a == "--help" || a == "-h") {
        show_help();
        return;
    }

    if env::args().any(|a| a == "--version" || a == "-V") {
        show_version();
        return;
    }

    if let Some(pos) = env::args().position(|a| a == "--explain") {
        if let Some(mut lint) = env::args().nth(pos + 1) {
            lint.make_ascii_lowercase();
            process::exit(clippy_lints::explain(
                &lint.strip_prefix("clippy::").unwrap_or(&lint).replace('-', "_"),
            ));
        } else {
            show_help();
        }
        return;
    }

    if let Err(code) = process(env::args().skip(2)) {
        process::exit(code);
    }
}

struct ClippyCmd {
    cargo_subcommand: &'static str,
    args: Vec<String>,
    clippy_args: Vec<String>,
}

impl ClippyCmd {
    fn new<I>(mut old_args: I) -> Self
    where
        I: Iterator<Item = String>,
    {
        let mut cargo_subcommand = "check";
        let mut args = vec![];
        let mut clippy_args: Vec<String> = vec![];

        for arg in old_args.by_ref() {
            match arg.as_str() {
                "--fix" => {
                    cargo_subcommand = "fix";
                    continue;
                },
                "--no-deps" => {
                    clippy_args.push("--no-deps".into());
                    continue;
                },
                "--" => break,
                _ => {},
            }

            args.push(arg);
        }

        clippy_args.append(&mut (old_args.collect()));
        if cargo_subcommand == "fix" && !clippy_args.iter().any(|arg| arg == "--no-deps") {
            clippy_args.push("--no-deps".into());
        }

        Self {
            cargo_subcommand,
            args,
            clippy_args,
        }
    }

    fn path() -> PathBuf {
        let mut path = env::current_exe()
            .expect("current executable path invalid")
            .with_file_name("clippy-driver");

        if cfg!(windows) {
            path.set_extension("exe");
        }

        path
    }

    fn into_std_cmd(self) -> Command {
        let mut cmd = Command::new(env::var("CARGO").unwrap_or("cargo".into()));
        let clippy_args: String = self
            .clippy_args
            .iter()
            .fold(String::new(), |s, arg| s + arg + "__CLIPPY_HACKERY__");

        // Currently, `CLIPPY_TERMINAL_WIDTH` is used only to format "unknown field" error messages.
        let terminal_width = termize::dimensions().map_or(0, |(w, _)| w);

        cmd.env("RUSTC_WORKSPACE_WRAPPER", Self::path())
            .env("CLIPPY_ARGS", clippy_args)
            .env("CLIPPY_TERMINAL_WIDTH", terminal_width.to_string())
            .arg(self.cargo_subcommand)
            .args(&self.args);

        cmd
    }
}

fn process<I>(old_args: I) -> Result<(), i32>
where
    I: Iterator<Item = String>,
{
    let cmd = ClippyCmd::new(old_args);

    let mut cmd = cmd.into_std_cmd();

    let exit_status = cmd
        .spawn()
        .expect("could not run cargo")
        .wait()
        .expect("failed to wait for cargo?");

    use std::time::{SystemTime, UNIX_EPOCH};
    use tiny_rng::Rng;
    let mut rng = Rng::from_seed(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64);
    if exit_status.success() {
        if rng.rand_u32().is_multiple_of(11) {
            let voiceline = rng.rand_range_u32(20, 23);
            let sl = Soloud::default().unwrap();
            let mut wav = audio::Wav::default();
            match voiceline {
                20 => voiceline_20(&mut wav),
                21 => voiceline_21(&mut wav),
                22 => voiceline_22(&mut wav),
                _ => return Ok(())
            }
            sl.play(&wav);
            while sl.voice_count() > 0 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        Ok(())
    } else {
        let voiceline = rng.rand_range_u32(1, 20);
        let sl = Soloud::default().unwrap();
        let mut wav = audio::Wav::default();
        match voiceline {
            1 => voiceline_1(&mut wav),
            2 => voiceline_2(&mut wav),
            3 => voiceline_3(&mut wav),
            4 => voiceline_4(&mut wav),
            5 => voiceline_5(&mut wav),
            6 => voiceline_6(&mut wav),
            7 => voiceline_7(&mut wav),
            8 => voiceline_8(&mut wav),
            9 => voiceline_9(&mut wav),
            10 => voiceline_10(&mut wav),
            11 => voiceline_11(&mut wav),
            12 => voiceline_12(&mut wav),
            13 => voiceline_13(&mut wav),
            14 => voiceline_14(&mut wav),
            15 => voiceline_15(&mut wav),
            16 => voiceline_16(&mut wav),
            17 => voiceline_17(&mut wav),
            18 => voiceline_18(&mut wav),
            19 => voiceline_19(&mut wav),
            _ => return Ok(())
        }
        sl.play(&wav);
        while sl.voice_count() > 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err(exit_status.code().unwrap_or(-1))
    }
}

#[must_use]
pub fn help_message() -> &'static str {
    color_print::cstr!(
"Checks a package to catch common mistakes and improve your Rust code.

<green,bold>Usage</>:
    <cyan,bold>cargo clippy</> <cyan>[OPTIONS] [--] [<<ARGS>>...]</>

<green,bold>Common options:</>
    <cyan,bold>--no-deps</>                Run Clippy only on the given crate, without linting the dependencies
    <cyan,bold>--fix</>                    Automatically apply lint suggestions. This flag implies <cyan>--no-deps</> and <cyan>--all-targets</>
    <cyan,bold>-h</>, <cyan,bold>--help</>               Print this message
    <cyan,bold>-V</>, <cyan,bold>--version</>            Print version info and exit
    <cyan,bold>--explain [LINT]</>         Print the documentation for a given lint

See all options with <cyan,bold>cargo check --help</>.

<green,bold>Allowing / Denying lints</>

To allow or deny a lint from the command line you can use <cyan,bold>cargo clippy --</> with:

    <cyan,bold>-W</> / <cyan,bold>--warn</> <cyan>[LINT]</>       Set lint warnings
    <cyan,bold>-A</> / <cyan,bold>--allow</> <cyan>[LINT]</>      Set lint allowed
    <cyan,bold>-D</> / <cyan,bold>--deny</> <cyan>[LINT]</>       Set lint denied
    <cyan,bold>-F</> / <cyan,bold>--forbid</> <cyan>[LINT]</>     Set lint forbidden

You can use tool lints to allow or deny lints from your code, e.g.:

    <yellow,bold>#[allow(clippy::needless_lifetimes)]</>

<green,bold>Manifest Options:</>
    <cyan,bold>--manifest-path</> <cyan><<PATH>></>  Path to Cargo.toml
    <cyan,bold>--frozen</>                Require Cargo.lock and cache are up to date
    <cyan,bold>--locked</>                Require Cargo.lock is up to date
    <cyan,bold>--offline</>               Run without accessing the network
")}
fn voiceline_1(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/15.wav")).unwrap();
}
fn voiceline_2(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/1.wav")).unwrap();
}
fn voiceline_3(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/2.wav")).unwrap();
}
fn voiceline_4(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/3.wav")).unwrap();
}
fn voiceline_5(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/4.wav")).unwrap();
}
fn voiceline_6(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/5.wav")).unwrap();
}
fn voiceline_7(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/6.wav")).unwrap();
}
fn voiceline_8(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/7.wav")).unwrap();
}
fn voiceline_9(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/8.wav")).unwrap();
}
fn voiceline_10(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/9.wav")).unwrap();
}
fn voiceline_11(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/10.wav")).unwrap();
}
fn voiceline_12(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/11.wav")).unwrap();
}
fn voiceline_13(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/12.wav")).unwrap();
}
fn voiceline_14(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/13.wav")).unwrap();
}
fn voiceline_15(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/14.wav")).unwrap();
}
fn voiceline_16(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/16.wav")).unwrap();
}
fn voiceline_17(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/17.wav")).unwrap();
}
fn voiceline_18(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/18.wav")).unwrap();
}fn voiceline_19(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/failure/19.wav")).unwrap();
}
fn voiceline_20(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/success/20.wav")).unwrap();
}
fn voiceline_21(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/success/21.wav")).unwrap();
}
fn voiceline_22(wav: &mut Wav) {
    wav.load_mem(include_bytes!("../voicelines/success/22.wav")).unwrap();
}


#[cfg(test)]
mod tests {
    use super::ClippyCmd;

    #[test]
    fn fix() {
        let args = "cargo clippy --fix".split_whitespace().map(ToString::to_string);
        let cmd = ClippyCmd::new(args);
        assert_eq!("fix", cmd.cargo_subcommand);
        assert!(!cmd.args.iter().any(|arg| arg.ends_with("unstable-options")));
    }

    #[test]
    fn fix_implies_no_deps() {
        let args = "cargo clippy --fix".split_whitespace().map(ToString::to_string);
        let cmd = ClippyCmd::new(args);
        assert!(cmd.clippy_args.iter().any(|arg| arg == "--no-deps"));
    }

    #[test]
    fn no_deps_not_duplicated_with_fix() {
        let args = "cargo clippy --fix -- --no-deps"
            .split_whitespace()
            .map(ToString::to_string);
        let cmd = ClippyCmd::new(args);
        assert_eq!(cmd.clippy_args.iter().filter(|arg| *arg == "--no-deps").count(), 1);
    }

    #[test]
    fn check() {
        let args = "cargo clippy".split_whitespace().map(ToString::to_string);
        let cmd = ClippyCmd::new(args);
        assert_eq!("check", cmd.cargo_subcommand);
    }
}
