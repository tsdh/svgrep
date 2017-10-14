#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate regex;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

use clap::{App, Arg, ArgMatches};
use regex::Regex;

#[derive(Debug)]
struct CSVFile {
    rows: Vec<CSVRow>,
    separator: String,
}

#[derive(Debug)]
struct CSVRow {
    cells: Vec<String>,
}

impl CSVRow {
    fn parse_line(line: String, sep: &str) -> CSVRow {
        CSVRow { cells: line.split(sep).map(|s| String::from(s)).collect() }
    }

    fn print(&self, cols: Vec<usize>, sep: &str) {
        for i in cols {
            if i >= self.cells.len() {
                print!("<no col {}>", i);
            } else {
                print!("{}", self.cells[i]);
            }
            print!("{}", sep);
        }
        println!();
    }
}

lazy_static! {
    static ref MATCH_OPT_RX: Regex = Regex::new(r"^(\*|\d+)~(.*)$").expect("Invalid Regex in the code!");
    static ref ALL_DIGIT_RX: Regex = Regex::new(r"^(\d+)$").expect("Invalid Regex in the code!");
}

impl CSVFile {
    fn parse_file(file_name: &str, sep: &str) -> CSVFile {
        let file = match File::open(file_name) {
            Ok(file) => file,
            Err(e) => panic!("{} when trying to read {}", e, file_name),
        };
        let buf_reader = BufReader::new(&file);
        CSVFile {
            rows: buf_reader
                .lines()
                .map(|l| CSVRow::parse_line(l.unwrap(), sep))
                .collect(),
            separator: String::from(sep),
        }
    }

    // TODO: Add args what to match
    fn match_and_print(&self, match_exp: &str) {
        if !MATCH_OPT_RX.is_match(match_exp) {
            panic!("{} is no valid match expression", match_exp)
        }

        let c = MATCH_OPT_RX.captures(match_exp).unwrap();
        let col_str = c.get(1).unwrap().as_str();

        if col_str != "*" && !ALL_DIGIT_RX.is_match(col_str) {
            panic!("{} is no valid column expression", col_str)
        }

        let rx = Regex::new(c.get(2).unwrap().as_str());

        // TODO: Now use them...

        for row in &self.rows {
            row.print(vec![0, 1, 2, 3], self.separator.as_str());
        }
    }
}

fn main() {
    let opts = parse_command_line();
    let sep = opts.value_of(OPT_SEPARATOR).unwrap_or(";");
    let csv_file = CSVFile::parse_file(opts.value_of(OPT_INPUT_FILE).unwrap(), sep);
    let match_val = opts.value_of(OPT_MATCH).unwrap_or(r"*~.*");

    csv_file.match_and_print(match_val);
}


const OPT_INPUT_FILE: &'static str = "INPUT_FILE";
const OPT_SEPARATOR: &'static str = "separator";
const OPT_MATCH: &'static str = "match";

fn parse_command_line<'a>() -> ArgMatches<'a> {
    App::new("svgrep -- Separated Values Grep")
        .version("0.1.0")
        .about("Greps and extracts cells of CSV/TSV/*SV files")
        .author("Tassilo Horn <tsdh@gnu.org>")
        .arg(
            Arg::with_name(OPT_INPUT_FILE)
                .help("The separated values file")
                .required(true),
        )
        .arg(
            Arg::with_name(OPT_SEPARATOR)
                .short("s")
                .long(OPT_SEPARATOR)
                .help("Sets the separator to be used (default: ';')")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(OPT_MATCH)
                .short("m")
                .long(OPT_MATCH)
                .takes_value(true)
                .help(
                    format!(
                        "{}\n{}\n{}",
                        "Sets a match expression of the form <col>~<regex>",
                        "<col> is a natural number or * meaning any column",
                        "<regex> is a regular expression matched against the cells at column <col>"
                    ).as_str(),
                ),
        )
        .get_matches()
}
