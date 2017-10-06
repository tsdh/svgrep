extern crate clap;
use clap::{App, Arg, ArgMatches};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

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
    fn match_and_print(&self) {
        for row in &self.rows {
            row.print(vec![0, 1, 2, 3], self.separator.as_str());
        }
    }
}

fn parse_command_line<'a>() -> ArgMatches<'a> {
    App::new("svgrep -- Separated Values Grep")
        .version("0.1.0")
        .about("Greps and extracts cells of CSV/TSV/*SV files")
        .author("Tassilo Horn <tsdh@gnu.org>")
        .arg(
            Arg::with_name("INPUT_FILE")
                .help("The separated values file")
                .required(true),
        )
        .arg(
            Arg::with_name("separator")
                .short("s")
                .long("separator")
                .help("Sets the separator to be used (default: ';')")
                .takes_value(true),
        )
        .get_matches()
}

fn main() {
    let opts = parse_command_line();
    let sep = opts.value_of("separator").unwrap_or(";");
    let csv_file = CSVFile::parse_file(opts.value_of("INPUT_FILE").unwrap(), sep);

    csv_file.match_and_print();
}
