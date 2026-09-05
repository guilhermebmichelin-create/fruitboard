use std::io::{self, BufRead, Write};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

const EXPECTED_PATH_MARKERS: [&str; 2] = [" ", "音"];

fn arguments() -> Result<(String, String), ()> {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() != Some("--mode") {
        return Err(());
    }
    let mode = arguments.next().ok_or(())?;
    if arguments.next().as_deref() != Some("--probe-path") {
        return Err(());
    }
    let probe_path = arguments.next().ok_or(())?;
    if arguments.next().is_some()
        || !EXPECTED_PATH_MARKERS
            .iter()
            .all(|marker| probe_path.contains(marker))
    {
        return Err(());
    }
    Ok((mode, probe_path))
}

fn respond() -> Result<(), ()> {
    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).map_err(|_| ())?;
    if input.trim_end() != "ping" {
        return Err(());
    }
    println!("pong:path-accepted");
    Ok(())
}

fn wait_for_termination() -> ! {
    println!("ready:path-accepted");
    io::stdout().flush().ok();
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn main() -> ExitCode {
    let Ok((mode, _probe_path)) = arguments() else {
        eprintln!("invalid_arguments");
        return ExitCode::from(2);
    };

    match mode.as_str() {
        "respond" => match respond() {
            Ok(()) => ExitCode::SUCCESS,
            Err(()) => {
                eprintln!("invalid_input");
                ExitCode::from(3)
            }
        },
        "fail" => {
            eprintln!("controlled_failure");
            ExitCode::from(17)
        }
        "wait" => wait_for_termination(),
        _ => {
            eprintln!("invalid_mode");
            ExitCode::from(2)
        }
    }
}
