// Copyright (c) 2021, Facebook, Inc.
// All rights reserved.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

#[derive(Debug)]
enum Error {
    IoError(std::io::Error),
    DepgraphError(String),
    Other(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Error::IoError(ref e) => ::std::fmt::Display::fmt(e, f),
            Error::DepgraphError(ref e) => f.write_str(e),
            Error::Other(ref e) => f.write_str(e),
        }
    }
}
impl std::error::Error for Error {}
impl std::convert::From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}
type Result<T> = std::result::Result<T, Error>;

/// Auxiliary function to print a 64-bit edge
fn print_edge_u64(key: u64, dst: u64, hex_dump: bool) {
    const DIGITS: usize = 20; // (u64::MAX as f64).log10() as usize + 1;

    if hex_dump {
        println!("  {key:#016x} {:#016x}", dst, key = key);
    } else {
        println!("  {key:>width$}  {}", dst, key = key, width = DIGITS);
    }
}

/// Add edges to `es` given source vertex `k` and dest vertices `vs`.
fn add_edges<T: Ord + Clone>(es: &mut Vec<(T, T)>, k: T, vs: &std::collections::BTreeSet<T>) {
    es.extend(vs.iter().map(|v| (k.clone(), v.clone())));
}

/// Retrieve the adjacency list for `k` in `g`.
///
/// This is the analog of `value_vertex` for 64-bit depgraphs.
fn hashes(g: &depgraph::reader::DepGraph<'_>, k: u64) -> std::collections::BTreeSet<u64> {
    match g.hash_list_for(depgraph::dep::Dep::new(k)) {
        None => std::collections::BTreeSet::<u64>::new(),
        Some(hashes) => g.hash_list_hashes(hashes).map(|x| x.into()).collect(),
    }
}

/// Print an ASCII representation of a 64-bit depgraph to stdout.
fn dump_depgraph64(file: &str, dependency_hash: Option<u64>, hex_dump: bool) -> Result<()> {
    let o = depgraph::reader::DepGraphOpener::from_path(file)?;
    let dg = o.open().map_err(Error::DepgraphError)?;
    let () = dg.validate_hash_lists().map_err(Error::DepgraphError)?;

    let print_edges_for_key = |key: u64| {
        let dests = hashes(&dg, key);
        for dst in dests {
            print_edge_u64(key, dst, hex_dump);
        }
    };

    match dependency_hash {
        None => {
            for &key in dg.all_hashes().iter() {
                print_edges_for_key(key)
            }
        }
        Some(dependency_hash) => print_edges_for_key(dependency_hash),
    };

    Ok(())
}

/// Compare two 64-bit dependency graphs.
///
/// Calculate the edges in `control_file` not in `test_file` (missing edges) and
/// the edges in `test_file` not in `control_file` (extraneous edges).
fn comp_depgraph64(
    no_progress_bar: bool,
    test_file: &str,
    control_file: &str,
    hex_dump: bool,
) -> Result<()> {
    let lo = depgraph::reader::DepGraphOpener::from_path(test_file)?;
    let ro = depgraph::reader::DepGraphOpener::from_path(control_file)?;
    let mut num_edges_missing = 0;
    match (|| {
        let (ldg, rdg) = (lo.open()?, ro.open()?);
        let ((), ()) = (ldg.validate_hash_lists()?, rdg.validate_hash_lists()?);
        let (lvs, rvs) = (ldg.all_hashes(), rdg.all_hashes());
        let (lnum_keys, rnum_keys) = (lvs.len(), rvs.len());
        let (mut lproc, mut rproc) = (0, 0);
        let (mut missing, mut extra) = (vec![], vec![]);
        let (mut lrows, mut rrows) = (lvs.iter(), rvs.iter());
        let (mut lro, mut rro) = (lrows.next(), rrows.next());
        let (mut ledge_count, mut redge_count) = (0, 0);
        let bar = if no_progress_bar {
            None
        } else {
            Some(indicatif::ProgressBar::new(
                std::cmp::max(lnum_keys, rnum_keys) as u64,
            ))
        };
        if let Some(bar) = bar.as_ref() {
            bar.println("Comparing graphs. Patience...")
        };
        while lro.is_some() || rro.is_some() {
            match (lro, rro) {
                (None, Some(&rk)) => {
                    // These edges are in `r` and not in `l`.
                    let k = rk;
                    let vs = hashes(&rdg, k);
                    redge_count += vs.len();
                    add_edges(&mut missing, k, &vs);
                    rro = rrows.next();
                    rproc += 1;
                    if bar.is_some() && rnum_keys > lnum_keys {
                        bar.as_ref().unwrap().inc(1); // We advanced `r` and there are more keys in `r` than `l`.
                    }
                }
                (Some(&lk), None) => {
                    // These edges are in `l` and not in `r`.
                    let k = lk;
                    let vs = hashes(&ldg, k);
                    lro = lrows.next();
                    ledge_count += vs.len();
                    add_edges(&mut extra, k, &vs);
                    lproc += 1;
                    if bar.is_some() && lnum_keys > rnum_keys {
                        bar.as_ref().unwrap().inc(1); // We advanced `l` and there are more keys in `l` than `r`.
                    }
                }
                (Some(&lk), Some(&rk)) => {
                    let (lvs, rvs) = (hashes(&ldg, lk), hashes(&rdg, rk));
                    if lk < rk {
                        // These edges are in `l` but not in `r`.
                        ledge_count += lvs.len();
                        add_edges(&mut extra, lk, &lvs);
                        lro = lrows.next();
                        lproc += 1;
                        if bar.is_some() && lnum_keys >= rnum_keys {
                            bar.as_ref().unwrap().inc(1); // We advanced `l` and there are more keys in `l` than `r`.
                        }
                        continue;
                    }
                    if lk > rk {
                        // These edges are in `r` but not in `l`.
                        redge_count += rvs.len();
                        add_edges(&mut missing, rk, &rvs);
                        rro = rrows.next();
                        rproc += 1;
                        if bar.is_some() && rnum_keys > lnum_keys {
                            bar.as_ref().unwrap().inc(1); // We advanced `r` and there are more keys in `r` than `l`.
                        }
                        continue;
                    }
                    ledge_count += lvs.len();
                    redge_count += rvs.len();
                    // Vertices in `rvs` not in `lvs` indicate missing edges.
                    let mut dests: std::collections::BTreeSet<u64> =
                        std::collections::BTreeSet::new();
                    dests.extend(rvs.iter().filter(|&v| !lvs.contains(v)));
                    add_edges(&mut missing, lk, &dests);
                    // Vertices in `lvs` not in `rvs` indicate extra edges.
                    dests.clear();
                    dests.extend(lvs.iter().filter(|&v| !rvs.contains(v)));
                    add_edges(&mut extra, lk, &dests);
                    lro = lrows.next();
                    rro = rrows.next();
                    lproc += 1;
                    rproc += 1;
                    if bar.is_some() {
                        bar.as_ref().unwrap().inc(1)
                    }; // No matter whether `l` or `r` has more keys, progress was made.
                }
                (None, None) => panic!("The impossible happened!"),
            }
        }
        if let Some(bar) = bar {
            bar.finish_and_clear()
        };
        num_edges_missing = missing.len();
        println!("\nResults\n=======");
        println!("Processed {}/{} of nodes in 'test'", lproc, lnum_keys);
        println!("Processed {}/{} of nodes in 'control'", rproc, rnum_keys);
        println!("Edges in 'test': {}", ledge_count);
        println!("Edges in 'control': {}", redge_count);
        println!(
            "Edges in 'control' missing in 'test' (there are {}):",
            missing.len()
        );
        for (key, dst) in missing {
            print_edge_u64(key, dst, hex_dump);
        }
        println!(
            "Edges in 'test' missing in 'control' (there are {}):",
            extra.len()
        );
        for (key, dst) in extra {
            print_edge_u64(key, dst, hex_dump);
        }
        Ok(())
    })() {
        Ok(()) => {
            if num_edges_missing == 0 {
                Ok(())
            } else {
                // Rust 2018 semantics are such that this will result in a
                // non-zero error code
                // (https://doc.rust-lang.org/edition-guide/rust-2018/error-handling-and-panics/question-mark-in-main-and-tests.html).
                Err(Error::Other(format!(
                    "{} missing edges detected",
                    num_edges_missing
                )))
            }
        }
        Err(msg) => Err(Error::DepgraphError(msg)),
    }
}

use structopt::StructOpt;

fn parse_hex_or_decimal(src: &str) -> std::result::Result<u64, std::num::ParseIntError> {
    let src_trim = src.trim_start_matches("0x");
    if src_trim.len() != src.len() {
        u64::from_str_radix(src_trim, 16)
    } else {
        src_trim.parse::<u64>()
    }
}

#[derive(Debug, structopt::StructOpt)]
#[structopt(
    name = "dump_saved_state_depgraph",
    about = "
Common usage is to provide two file arguments to compare, 'test' and 'control'.

Example invocation:

  dump_saved_state_depgraph --bitness 32 \\
      --test path/to/test.bin --control path/to/control.bin

Exit code will be 0 if 'test' >= 'control' and 1 if 'test' < 'control'."
)]
struct Opt {
    #[structopt(long = "with-progress-bar", help = "Enable progress bar display")]
    with_progress_bar: bool,

    #[structopt(long = "dump", help = "graph to render as text")]
    dump: Option<String>,

    #[structopt(
        long = "dependency-hash",
        help = "(with --dump; only for 64-bit) only dump edges for the given dependency hash",
        parse(try_from_str = parse_hex_or_decimal)
    )]
    dependency_hash: Option<u64>,

    #[structopt(long = "print-hex", help = "print hexadecimal hashes")]
    print_hex: bool,

    #[structopt(long = "test", help = "'test' graph")]
    test: Option<String>,

    #[structopt(long = "control", help = "'control' graph")]
    control: Option<String>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    match match opt {
        Opt {
            dump: Some(file),
            dependency_hash,
            print_hex,
            ..
        } => dump_depgraph64(&file, dependency_hash, print_hex),
        Opt {
            with_progress_bar,
            test: Some(test),
            control: Some(control),
            print_hex,
            ..
        } => comp_depgraph64(!with_progress_bar, &test, &control, print_hex),
        _ => Ok(()),
    } {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
