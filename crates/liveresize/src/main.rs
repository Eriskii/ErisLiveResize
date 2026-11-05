//! Installs and runs the root daemon that loads the payload into every app of the logged-in user as
//! it starts (through eris-inject's daemon loop).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use eris_inject::daemon::Client;
use eris_inject::inject::{self, Payload};
use eris_inject::launchd::Service;
use eris_inject::{log, procs};

const PAYLOAD: &str = "libliveresize.dylib";

const SERVICE: Service = Service {
    label: "io.github.eriskii.liveresize",
    dir: "/Library/Application Support/ErisLiveResize",
    log: "/Library/Logs/liveresize.log",
    files: &[("liveresize", 0o755), (inject::THREAD_HELPER, 0o755), (PAYLOAD, 0o644)],
    arguments: &["daemon"],
    link: Some("/usr/local/bin/liveresize"),
};

struct LiveResize {
    payload: PathBuf,
}

impl Client for LiveResize {
    fn wants(&mut self, app: &procs::App, _home: &Path) -> bool {
        matches!(app.kind, procs::Kind::Regular | procs::Kind::Agent)
            && !app.system
            && !(app.nested && app.in_frameworks)
            && !eris_inject::daemon::ALWAYS_SKIPPED.contains(&app.id.as_str())
    }

    fn load(&mut self, pid: i32, _bundle_id: &str) -> Result<(), inject::Error> {
        inject::inject(pid, &Payload { path: &self.payload, name: "liveresize", patches: &[] })
    }
}

fn run(command: &str) -> Result<(), String> {
    match command {
        "install" => {
            SERVICE.install()?;
            println!("installed; the daemon is loading into your apps now");
        }
        "uninstall" => {
            SERVICE.uninstall()?;
            println!("uninstalled; apps go back to normal as they're relaunched");
        }
        "status" => match (SERVICE.installed(), SERVICE.pid()) {
            (_, Some(pid)) => println!("daemon: running (pid {pid})"),
            (true, None) => println!("daemon: installed but not running; see {}", SERVICE.log),
            (false, None) => println!("daemon: not installed (sudo liveresize install)"),
        },
        "daemon" => {
            let payload = Path::new(SERVICE.dir).join(PAYLOAD);
            log!("started; payload {}", payload.display());
            eris_inject::daemon::run(&mut LiveResize { payload })
        }
        _ => return Err("usage: liveresize install | uninstall | status".to_string()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let command = std::env::args().nth(1).unwrap_or_default();
    match run(&command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
