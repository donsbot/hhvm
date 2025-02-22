// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.
// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

use std::path::PathBuf;
use std::sync::Arc;

use indicatif::ParallelProgressIterator;
use jwalk::WalkDir;
use rayon::prelude::*;
use structopt::StructOpt;

use hackrs::decl_parser::DeclParser;
use hackrs::reason::{BReason, NReason, Reason};
use hackrs::shallow_decl_provider::ShallowDeclCache;
use hackrs_test_utils::cache::NonEvictingCache;
use hackrs_test_utils::serde_cache::{Compression, SerializingCache};
use pos::{Prefix, RelativePath, RelativePathCtx};

#[derive(StructOpt, Debug)]
struct CliOptions {
    /// The repository root (where .hhconfig is), e.g., ~/www
    root: PathBuf,

    /// Allocate decls with positions instead of allocating position-free decls.
    #[structopt(long)]
    with_pos: bool,

    /// Store decls in a data store which serializes and compresses them.
    #[structopt(long)]
    serialize: bool,

    /// If `--serialize` was given, use the given compression algorithm.
    #[structopt(default_value, long)]
    compression: Compression,
}

fn main() {
    let opts = CliOptions::from_args();

    let path_ctx = Arc::new(RelativePathCtx {
        root: opts.root.clone(),
        hhi: PathBuf::new(),
        dummy: PathBuf::new(),
        tmp: PathBuf::new(),
    });

    let (filenames, time_taken) = time(|| {
        WalkDir::new(opts.root.clone())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_type().is_dir() && find_utils::is_hack(&e.path()))
            .map(|e| {
                let path = e.path();
                match path.strip_prefix(&path_ctx.root) {
                    Ok(suffix) => RelativePath::new(Prefix::Root, suffix),
                    Err(..) => RelativePath::new(Prefix::Dummy, &path),
                }
            })
            .collect::<Vec<RelativePath>>()
    });
    println!(
        "Collected {} filenames in {:?}",
        filenames.len(),
        time_taken
    );

    if opts.with_pos {
        parse_repo::<BReason>(&opts, path_ctx, &filenames);
    } else {
        parse_repo::<NReason>(&opts, path_ctx, &filenames);
    }
}

fn parse_repo<R: Reason>(opts: &CliOptions, ctx: Arc<RelativePathCtx>, filenames: &[RelativePath]) {
    let decl_parser = DeclParser::<R>::new(ctx);
    let shallow_decl_cache = if opts.serialize {
        ShallowDeclCache::new(
            Arc::new(SerializingCache::with_compression(opts.compression)), // types
            Box::new(SerializingCache::with_compression(opts.compression)), // funs
            Box::new(SerializingCache::with_compression(opts.compression)), // consts
            Box::new(SerializingCache::with_compression(opts.compression)), // properties
            Box::new(SerializingCache::with_compression(opts.compression)), // static_properties
            Box::new(SerializingCache::with_compression(opts.compression)), // methods
            Box::new(SerializingCache::with_compression(opts.compression)), // static_methods
            Box::new(SerializingCache::with_compression(opts.compression)), // constructors
        )
    } else {
        ShallowDeclCache::with_no_member_caches(
            Arc::new(NonEvictingCache::default()),
            Box::new(NonEvictingCache::default()),
            Box::new(NonEvictingCache::default()),
        )
    };
    let ((), time_taken) = time(|| {
        filenames
            .par_iter()
            .progress_count(filenames.len() as u64)
            .for_each(|&path| shallow_decl_cache.add_decls(decl_parser.parse(path).unwrap()))
    });
    println!("Parsed {} files in {:?}", filenames.len(), time_taken);

    let me = procfs::process::Process::myself().unwrap();
    let page_size = procfs::page_size().unwrap();
    println!(
        "RSS: {:.3}GiB",
        (me.stat.rss * page_size) as f64 / 1024.0 / 1024.0 / 1024.0
    );

    // exit without running destructors (our cache is huge and full of Arcs)
    std::process::exit(0);
}

fn time<T>(f: impl FnOnce() -> T) -> (T, std::time::Duration) {
    let start = std::time::Instant::now();
    let result = f();
    let time_taken = start.elapsed();
    (result, time_taken)
}
