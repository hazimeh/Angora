#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate angora;
extern crate angora_common;

use angora::executor::Forksrv;
use angora_common::defs;
use angora::{branches};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::Write,
    os::unix::io::RawFd,
    path::PathBuf,
    sync::Arc,
};

fn main() {
    let matches = App::new("angora-showmap")
        .version(crate_version!())
        .about("Displays the contents of the trace bitmap.")
        .arg(Arg::with_name("output_file")
             .short("o")
             .long("output")
             .value_name("FILE")
             .help("File to write the trace data to")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("at_file")
             .short("A")
             .long("at_file")
             .value_name("INPUT_FILE")
             .help("The path to the file which substitutes @@ in the command args")
             .takes_value(true))
        .arg(Arg::with_name("time_limit")
             .short("T")
             .long("time_limit")
             .value_name("TIME")
             .help("Time limit for each run, default is 1(s)")
             .takes_value(true))
        .arg(Arg::with_name("memory_limit")
             .short("M")
             .long("memory_limit")
             .value_name("MEM")
             .help("Memory limit for programs, default is 200(MB), set 0 for unlimited memory")
             .takes_value(true))
        .arg(Arg::with_name("branch_only")
             .short("b")
             .long("branch_only")
             .help("Show branch coverage only, ignore hit counts"))
        .arg(Arg::with_name("cmin_mode")
            .short("Z")
            .long("cmin_mode")
            .help("Output the syntax expected by angora-cmin"))
        .arg(Arg::with_name("pargs")
            .help("Targeted program and arguments")
            .required(true)
            .multiple(true)
            .allow_hyphen_values(true)
            .last(true)
            .index(1))
        .get_matches();

    let branch_only = matches.occurrences_of("branch_only") > 0;
    let cmin_mode = matches.occurrences_of("cmin_mode") > 0;
    let at_file = matches.value_of("at_file");
    let mut is_stdin = true;

    let pargs = matches.values_of_lossy("pargs").unwrap();
    let prog_bin = pargs[0].clone();
    let mut prog_args = pargs[1..].to_vec();

    let at_pos = prog_args.iter().position(|x| x == "@@");
    if ! at_pos.is_none() {
        if at_file.is_none() {
            panic!("@@ is not supported without -A/--at_file");
        }
        let subst_file = PathBuf::from(at_file.unwrap()).canonicalize().unwrap().into_os_string().into_string().unwrap();
        prog_args[at_pos.unwrap()] = subst_file;
        is_stdin = false;
    }

    let global_branches = Arc::new(branches::GlobalBranches::new());
    let branches = branches::Branches::new(global_branches);

    let mut envs = HashMap::new();
    envs.insert(
        defs::BRANCHES_SHM_ENV_VAR.to_string(),
        branches.get_id().to_string(),
    );

    let out_file_path = matches.value_of("output_file").unwrap();
    let mut out_file = match File::create(out_file_path) {
        Ok(file) => file,
        Err(err) => panic!("could not open {:?}: {:?}", out_file_path, err),
    };

    let mut forksrv = Forksrv::new(
        "/tmp/angora_showmap",
        &(prog_bin, prog_args),
        &envs,
        0 as RawFd,
        is_stdin,
        false,
        value_t!(matches, "time_limit", u64).unwrap_or(angora_common::config::TIME_LIMIT),
        value_t!(matches, "memory_limit", u64).unwrap_or(angora_common::config::MEM_LIMIT),
    );
    forksrv.run();
    let path = branches.get_path();

    for (idx, mut count) in path {
        count = if branch_only {
            1
        } else {
            count
        };
        if cmin_mode {
            writeln!(out_file, "{}{}", count, idx).unwrap();
        } else {
            writeln!(out_file, "{}:{}", idx, count).unwrap();
        }
    };
}
