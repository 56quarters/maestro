//
//
//

//!

extern crate blag;
#[macro_use]
extern crate clap;
extern crate libc;
extern crate nix;

use clap::{App, Arg, ArgMatches};
use libc::pid_t;
use nix::unistd::Pid;
use std::env;
use std::process::{self, Command};
use std::sync::Arc;

use blag::{ChildPid, SignalCatcher, SignalHandler, ThreadMasker, SIGNALS_TO_HANDLE};

fn parse_cli_opts<'a>(args: Vec<String>) -> ArgMatches<'a> {
    App::new("PID 1")
        .version(crate_version!())
        .set_term_width(72)
        .about("\nIt does PID 1 things")
        .arg(Arg::with_name("command").multiple(true).help(
            "Command to execute and arguments to it. Note that the command \
             must be an absolute path. For example `/usr/bin/whatever`, not just \
             `whatever`. Any arguments to pass to the command should be listed as \
             well, separated with spaces.",
        ))
        .get_matches_from(args)
}

fn main() {
    let matches = parse_cli_opts(env::args().collect());
    let arguments = values_t!(matches, "command", String).unwrap_or_else(|e| e.exit());

    let masker = ThreadMasker::new(SIGNALS_TO_HANDLE);
    masker.block_for_thread();

    let catcher = SignalCatcher::new(SIGNALS_TO_HANDLE);
    let receiver = catcher.launch();

    let pid = Arc::new(ChildPid::default());
    let pid_clone = Arc::clone(&pid);

    let handler = SignalHandler::new(receiver, pid_clone, SIGNALS_TO_HANDLE);
    handler.launch();

    let mut child = match Command::new(&arguments[0]).args(&arguments[1..]).spawn() {
        Err(e) => {
            eprintln!("blag: command error: {}", e);
            process::exit(1);
        }
        Ok(c) => c,
    };

    pid.set_pid(Pid::from_raw(child.id() as pid_t));
    let status = match child.wait() {
        Err(e) => {
            eprintln!("blag: wait error: {}", e);
            process::exit(1);
        }
        Ok(s) => s,
    };

    if let Some(code) = status.code() {
        process::exit(code);
    } else {
        process::exit(0);
    }
}
