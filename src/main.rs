// A grep-like tool for separated values files.
//
// Copyright (C) 2017  Tassilo Horn <tsdh@gnu.org>
//
// This program is free software; you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation; either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program; if not, write to the Free Software Foundation, Inc., 51
// Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA

#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate regex;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};
use std::process::exit;

use clap::{App, Arg, ArgMatches};
use regex::Regex;

struct CSVRow {
    cells: Vec<String>,
}

enum CellSelect {
    ALL,
    Some(Vec<usize>),
}

struct MatchExp {
    rxs: Vec<Regex>,
    cell_rxs: HashMap<usize, Regex>,
    sel: CellSelect,
}

struct Config {
    separator: String,
    trim: bool,
    match_exps: Vec<MatchExp>,
}

struct MatchCharCfg {
    cell_select_char: String,
    match_conj_char: String,
    matches_char: String,
}

impl MatchExp {
    fn empty() -> MatchExp {
        MatchExp {
            rxs: vec![],
            cell_rxs: HashMap::new(),
            sel: CellSelect::ALL,
        }
    }

    fn match_and_select(&self, row: &CSVRow, config: &Config) {
        let mut row_matches = self.rxs.is_empty() && self.cell_rxs.is_empty();

        row_matches = row_matches
            || self.cell_rxs.iter().all(|(cell_idx, rx)| {
                let cell = row.get_cell(*cell_idx);
                cell.is_some() && rx.is_match(cell.unwrap())
            });
        row_matches = row_matches
            && self
                .rxs
                .iter()
                .all(|rx| row.cells.iter().any(|cell| rx.is_match(cell)));

        if row_matches {
            row.print(&self.sel, config);
        }
    }
}

impl CSVRow {
    fn parse_line(line: String, sep: &str) -> CSVRow {
        CSVRow {
            cells: line.split(sep).map(|s| String::from(s)).collect(),
        }
    }

    fn get_cell(&self, idx: usize) -> Option<&str> {
        if idx >= self.cells.len() {
            None
        } else {
            Some(self.cells[idx].as_str())
        }
    }

    fn print(&self, cols: &CellSelect, config: &Config) {
        match cols {
            &CellSelect::ALL => {
                for (i, cell) in self.cells.iter().enumerate() {
                    print!(
                        "({}) {}{} ",
                        i,
                        maybe_trim(cell, config.trim),
                        config.separator
                    );
                }
            }
            &CellSelect::Some(ref cols) => {
                for i in cols {
                    if i >= &self.cells.len() {
                        print!("<no col {}>", i);
                    } else {
                        print!(
                            "({}) {}",
                            i,
                            maybe_trim(self.cells[*i].as_str(), config.trim)
                        );
                    }
                    print!("{} ", config.separator);
                }
            }
        }
        println!();
    }
}

lazy_static! {
    static ref NUMBER_RX: Regex = Regex::new(r"^\d+.*$").expect("Invalid Regex in the code!");
    static ref ASTERISK_RX: Regex =
        Regex::new([r"^", regex::escape("*").as_str(), "$"].join("").as_ref())
            .expect("Invalid Regex in the code!");
}

fn maybe_trim(cell: &str, trim: bool) -> &str {
    if trim {
        cell.trim()
    } else {
        cell
    }
}

fn line_iter(file_name: Option<&str>) -> Lines<Box<dyn BufRead>> {
    let reader: Box<dyn BufRead> = match file_name {
        None => Box::new(BufReader::new(io::stdin())),
        Some(filename) => Box::new(BufReader::new(
            File::open(filename).expect(format!("No file {}", filename).as_str()),
        )),
    };
    reader.lines()
}

fn svgrep_lines(lines: Lines<Box<dyn BufRead>>, config: Config) {
    let all_match = &vec![MatchExp::empty()];
    let match_exps = if config.match_exps.is_empty() {
        all_match
    } else {
        &config.match_exps
    };

    for row in lines.map(|l| CSVRow::parse_line(l.unwrap(), &config.separator)) {
        for match_exp in match_exps {
            match_exp.match_and_select(&row, &config);
        }
    }
}

fn error(msg: &str) {
    eprintln!("Error: {}", msg);
    exit(1);
}

fn build_rxs(
    m: Option<regex::Match>,
    match_char_cfg: &MatchCharCfg,
) -> (Vec<Regex>, HashMap<usize, Regex>) {
    match m {
        None => (vec![], HashMap::new()),
        Some(m) => {
            let match_clauses: Vec<&str> =
                m.as_str().split(&match_char_cfg.match_conj_char).collect();
            let mut v = Vec::new();
            let mut hm = HashMap::new();

            for clause in match_clauses {
                let col_and_rx: Vec<&str> = clause.split(&match_char_cfg.matches_char).collect();
                if NUMBER_RX.is_match(col_and_rx[0]) {
                    hm.insert(
                        col_and_rx[0]
                            .parse::<usize>()
                            .expect("Invalid match column!"),
                        Regex::new(col_and_rx[1]).expect("Invalid regex!"),
                    );
                } else if ASTERISK_RX.is_match(col_and_rx[0]) {
                    v.push(Regex::new(col_and_rx[1]).expect("Invalid regex!"));
                } else {
                    error(format!("'{}' is no valid column spec!", col_and_rx[0]).as_str());
                }
            }

            (v, hm)
        }
    }
}

fn build_cell_select(s: Option<regex::Match>) -> CellSelect {
    match s {
        None => CellSelect::ALL,
        Some(v) => CellSelect::Some(
            v.as_str()
                .split(",")
                .map(|is| is.parse::<usize>().expect("Invalid index in select!"))
                .collect(),
        ),
    }
}

fn build_match_exp(match_val: &str, match_char_cfg: &MatchCharCfg) -> MatchExp {
    let rx = Regex::new(
        [
            r"^([^",
            regex::escape(&match_char_cfg.cell_select_char).as_ref(),
            "]+)?(?:",
            regex::escape(&match_char_cfg.cell_select_char).as_ref(),
            r"(\d+(,\d+)*))?$",
        ]
        .join("")
        .as_ref(),
    )
    .unwrap();

    let captures = rx.captures(match_val).expect("Invalid --match expression!");

    let (rxs, cell_rxs) = build_rxs(captures.get(1), match_char_cfg);
    MatchExp {
        rxs: rxs,
        cell_rxs: cell_rxs,
        sel: build_cell_select(captures.get(2)),
    }
}

fn build_config(opts: &ArgMatches) -> Config {
    let match_char_cfg = MatchCharCfg {
        cell_select_char: String::from(opts.value_of(OPT_SELECT_CHAR).unwrap_or("@")),
        match_conj_char: String::from(opts.value_of(OPT_CONJ_CHAR).unwrap_or("&")),
        matches_char: String::from(opts.value_of(OPT_MATCHES_CHAR).unwrap_or("=")),
    };

    Config {
        separator: String::from(opts.value_of(OPT_SEPARATOR).unwrap_or(";")),
        trim: opts.is_present(OPT_TRIM),
        match_exps: opts
            .values_of(OPT_MATCH)
            .unwrap_or(clap::Values::default())
            .map(|match_val| build_match_exp(match_val, &match_char_cfg))
            .collect(),
    }
}

fn main() {
    let opts = parse_command_line();
    let config = build_config(&opts);

    let lines = line_iter(opts.value_of(OPT_FILE));
    svgrep_lines(lines, config);
}

const OPT_FILE: &'static str = "FILE";
const OPT_SEPARATOR: &'static str = "separator";
const OPT_MATCH: &'static str = "match";
const OPT_CONJ_CHAR: &'static str = "conj-char";
const OPT_SELECT_CHAR: &'static str = "cell-select-char";
const OPT_MATCHES_CHAR: &'static str = "matches-char";
const OPT_TRIM: &'static str = "trim";
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

fn parse_command_line<'a>() -> ArgMatches<'a> {
    App::new("svgrep -- Separated Values Grep")
        .version(VERSION.unwrap_or("<version unknown>"))
        .about("Greps and extracts cells of CSV/TSV/*SV files")
        .author("Tassilo Horn <tsdh@gnu.org>")
        .arg(
            Arg::with_name(OPT_FILE)
                .help("The separated values file. If none is given, reads from stdin.")
                .required(false),
        )
        .arg(
            Arg::with_name(OPT_SEPARATOR)
                .short("s")
                .long(OPT_SEPARATOR)
                .takes_value(true)
                .value_name("char")
                .help("Sets the separator to be used (default: ';')"),
        )
        .arg(
            Arg::with_name(OPT_MATCH)
                .short("m")
                .long(OPT_MATCH)
                .takes_value(true)
                .multiple(true)
                .help(
                    format!(
                        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
                        "Sets the match-and-select expression.\n",
                        "Syntax:\n<col>=<regex>(&<col>=<regex>)+@<disp_cols>",
                        "<col> is a natural number or * meaning any column.",
                        "<regex> is a regex matched against the cells at column <col>.",
                        "<disp_cols> is a comma-separated list of columns to display (defaul: all).",
                        "\n--match '1=foo&2=bar' acts as logical AND wheras multiple expressions like",
                        "--match '1=foo' '2=bar' act as a logical OR."
                    ).as_str(),
                ),
        )
        .arg(Arg::with_name(OPT_MATCHES_CHAR)
             .short("=")
             .long(OPT_MATCHES_CHAR)
             .takes_value(true)
             .value_name("char")
             .help(format!("{}\n{}",
                           "Separates a <col> from the <regex> in --match expressions.",
                           "(default: =).").as_str()))
        .arg(Arg::with_name(OPT_CONJ_CHAR)
             .short("&")
             .long(OPT_CONJ_CHAR)
             .takes_value(true)
             .value_name("char")
             .help(format!("{}\n{}",
                           "Separates multiple <col>=<regex> pairs in --match expressions",
                           "to form a conjunction (default: &).").as_str()))
        .arg(Arg::with_name(OPT_SELECT_CHAR)
             .short("@")
             .long(OPT_SELECT_CHAR)
             .takes_value(true)
             .value_name("char")
             .help(format!("{}\n{}",
                           "Separates the <col>=<regex> pairs in --match expressions from",
                           "the column display selection (default: @).").as_str()))
        .arg(Arg::with_name(OPT_TRIM)
             .short("t")
             .long(OPT_TRIM)
             .help("Trim the cell contents when printing."))
        .get_matches()
}
