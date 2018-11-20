// Maestro - A basic init process for use in containers
//
// Copyright 2018 TSH Labs
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//!

extern crate maestro;
#[macro_use]
extern crate clap;
extern crate libc;
extern crate nix;

use clap::{App, Arg, ArgMatches};
use libc::pid_t;
use nix::unistd::Pid;
use std::env;
use std::os::unix::process::ExitStatusExt;
use std::process::{self, Command};
use std::sync::Arc;

use maestro::{ChildPid, SignalCatcher, SignalHandler, ThreadMasker, SIGNALS_TO_HANDLE};

fn parse_cli_opts<'a>(args: Vec<String>) -> ArgMatches<'a> {
    App::new("Maestro")
        .version(crate_version!())
        .set_term_width(72)
        .about("\nBasic init process for use in a container")
        .arg(Arg::with_name("command").multiple(true).help(
            "Command to execute and arguments to it. Any arguments to pass to \
             the command should be listed as well, separated with spaces.",
        ))
        .get_matches_from(args)
}

fn main() {
    let matches = parse_cli_opts(env::args().collect());
    let arguments = values_t!(matches, "command", String).unwrap_or_else(|e| e.exit());

    // Block all signals in the current (main) thread
    let masker = ThreadMasker::new(SIGNALS_TO_HANDLE);
    masker.block_for_thread();

    // Spawn another thread that will catch all signals and send them to yet another
    // thread via a send/receive channel pair.
    let catcher = SignalCatcher::new(SIGNALS_TO_HANDLE);
    let receiver = catcher.launch();

    let pid = Arc::new(ChildPid::default());
    let pid_clone = Arc::clone(&pid);

    // Spawn a thread that will read each signal received from the channel and send
    // it to the child process.
    let handler = SignalHandler::new(receiver, pid_clone, SIGNALS_TO_HANDLE);
    handler.launch();

    // Actually launch the child process.
    let mut child = match Command::new(&arguments[0]).args(&arguments[1..]).spawn() {
        Err(e) => {
            eprintln!("maestro: command error: {}", e);
            process::exit(1);
        }
        Ok(c) => c,
    };

    // Set the PID of the child (needed for the "handler" since it was spawned before
    // we actually knew the PID of the child).
    pid.set_pid(Pid::from_raw(child.id() as pid_t));

    // Wait for the child to exit.
    let status = match child.wait() {
        Err(e) => {
            eprintln!("maestro: wait error: {}", e);
            process::exit(1);
        }
        Ok(s) => s,
    };

    if let Some(code) = status.code() {
        process::exit(code);
    } else if let Some(sig) = status.signal() {
        process::exit(128 + sig);
    } else {
        process::exit(0);
    }
}
