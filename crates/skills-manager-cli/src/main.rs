use serde_json::json;

mod args;
mod commands;
mod help;
mod output;
mod time;
mod util;

use args::CliArgs;
use output::{print_human, RunContext};

fn main() {
    let exit_code = match run() {
        Ok(exit_code) => exit_code,
        Err(err) => {
            eprintln!("error: {err}");
            1
        }
    };
    std::process::exit(exit_code);
}

fn run() -> skills_manager_core::Result<i32> {
    let args = CliArgs::parse(std::env::args().skip(1).collect());
    if args.command.is_empty() || args.command[0] == "help" {
        help::print_help();
        return Ok(0);
    }

    let mut run = RunContext::new(&args);
    let result = commands::dispatch(&args, &mut run);

    if args.json {
        match result {
            Ok(payload) => {
                run.finish_success(payload);
                run.persist();
                println!("{}", serde_json::to_string_pretty(&run.output()).unwrap());
                Ok(if run.ok { 0 } else { 1 })
            }
            Err(err) => {
                run.finish_error(&err);
                run.persist();
                println!("{}", serde_json::to_string_pretty(&run.output()).unwrap());
                Ok(1)
            }
        }
    } else {
        match result {
            Ok(payload) => {
                print_human(&args.command, payload);
                run.finish_success(json!({}));
                run.persist();
                Ok(if run.ok { 0 } else { 1 })
            }
            Err(err) => Err(err),
        }
    }
}
