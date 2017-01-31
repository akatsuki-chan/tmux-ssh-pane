extern crate libc;
#[macro_use]
extern crate nom;
extern crate liner;

use liner::Context;

use std::str;
use std::fs::File;
use std::io::{Read, Error};
use std::process::Command;

use nom::{multispace, space};

fn is_hostname(chr: char) -> bool {
    nom::is_alphanumeric(chr as u8) || chr == '-' || chr == '_' || chr == '.'
}

named!(hostname<&str, &str>, take_while!(is_hostname));

named!(host<&str, &str>,
    do_parse!(
        opt!(multispace)
        >> tag_no_case!("host ")
        >> many0!(space)
        >> host: hostname
        >> opt!(multispace)
        >>

        (host)
    )
);

named!(ssh<&str, &str>,
    do_parse!(
        name: host 
        >> skip_options 
        >>

        (name)
    )
);

named!(ssh_config<&str, Vec<&str>>, many0!(ssh));

fn skip_options(input: &str) -> nom::IResult<&str, ()> {
    let mut _input = input.clone();
    loop {
        let r = tag_no_case!(_input, "host ");
        match r {
            nom::IResult::Done(_, _) => {
                // ここまでスキップ
                return nom::IResult::Done(_input, ());
            }
            _ => {
                if _input.len() < 4 {
                    // 見つからん時は元のやつを返す
                    return nom::IResult::Done(input, ());
                }
                _input = &_input[1..]
            }
        }
    }
}

fn read_ssh_config(path: std::path::PathBuf) -> Result<String, std::io::Error> {
    let mut file = try!(File::open(path.as_os_str()));
    let mut s = String::new();
    let _ = file.read_to_string(&mut s);

    Ok(s)
}

#[allow(unused_must_use)]
fn main() {
    let path = std::env::home_dir()
        .map(|mut home| {
            home.push(".ssh");
            home.push("config");
            home
        })
        .unwrap();

    let s = read_ssh_config(path).unwrap();
    let mut con = Context::new();

    match ssh_config(s.as_ref()) {
        nom::IResult::Done(_, hosts) => {
            ssh_connect(&hosts).unwrap();

            loop {
                let res = con.read_line("[prompt]$ ", &mut |_| {}).unwrap();
                tmux_run(&["select-window", "-t", "tmux_rust_panels"]);

                if res.is_empty() {
                    // おしまい
                    tmux_exit(hosts);
                    break;
                }

                for n in 0..hosts.len() {
                    tmux_send_key(n, res.as_ref());
                }

                con.history.push(res.into());
            }
        }
        _ => panic!("parse failed."),
    }
}

fn tmux_send_key(pane: usize, command: &str) -> Result<(), Error> {
    tmux_run(&["select-pane", "-t", pane.to_string().as_str()])
        .and_then(|_| tmux_run(&["send-keys", command, "C-m"]))
}

fn tmux_exit(hosts: Vec<&str>) -> Result<(), Error> {
    for n in 0..hosts.len() {
        try!(tmux_run(&["select-pane", "-t", n.to_string().as_str()]));
        try!(tmux_run(&["send-keys", "C-d"]))
    }

    Ok(())
}

fn ssh_connect(hosts: &Vec<&str>) -> Result<(), Error> {
    tmux_run(&["new-window", "-n", "tmux_rust_panels"])
        .and_then(|_| tmux_run(&["send-keys", format!("ssh {}", hosts[0]).as_ref(), "C-m"]))
        .and_then(|_| {
            for h in &hosts[1..] {
                try!(tmux_run(&["split-window", "-v"]));
                try!(tmux_run(&["send-keys", format!("ssh {}", h.to_string()).as_ref(), "C-m"]));
            }

            Ok(())
        })
}

fn tmux_run(args: &[&str]) -> Result<(), Error> {
    try!(Command::new("tmux").args(args).output());
    Ok(())
}
