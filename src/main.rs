use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use clap::Parser;
use csv::Writer;
use eyre::{Context, Result};
use log::{info, warn};
use qq_group_name_extract::qqtable::Member;
use walkdir::WalkDir;

/// Program to extract QQ group names and related info from an html table pasted from `https://qun.qq.com/member.html`
#[derive(Parser, Debug)]
#[clap(about, long_about = None)]
struct Args {
    /// File or dir to be converted
    #[clap(required = true, parse(from_os_str), value_name = "FILE")]
    paths: Vec<PathBuf>,

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

fn main() -> Result<()> {
    let args = Args::parse();
    pretty_env_logger::env_logger::Builder::new()
        // .filter_level(args.verbose.log_level_filter())
        .filter_module("qq_group_name_extract", args.verbose.log_level_filter())
        .init();

    let paths = args.paths;

    info!("Given path: {:?}", paths);

    for path in paths {
        for path in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_owned())
            .filter(|p| p.is_file() && p.extension().unwrap() == "html")
        {
            convert_html(&path)
                .wrap_err_with(|| format!("Error while converting to html: {path:?}"))?;
        }
    }

    Ok(())
}

fn convert_html<T: AsRef<Path>>(path: T) -> Result<()> {
    let path = path.as_ref();

    info!("Converting path: {path:?}");

    let mut file_str = String::new();
    File::open(path)
        .wrap_err_with(|| format!("Failed to open file {path:?}"))?
        .read_to_string(&mut file_str)
        .wrap_err_with(|| format!("Failed to read file {path:?}"))?;

    let table = Member::from_html(&file_str)
        .wrap_err_with(|| format!("Error while parsing file {path:?}"))?;

    let out_path = path.with_extension("csv");
    if out_path.is_file() {
        warn!("Overwriting file {out_path:?}");
    }

    let mut wtr = Writer::from_path(&out_path)
        .wrap_err_with(|| format!("Failed to create csv writer for file {out_path:?}"))?;
    // let writer = BufWriter::new(File::create(out_path)?);

    wtr.write_record(&[
        // "id",
        "成员",
        "群昵称",
        "QQ号",
        "性别",
        "Q龄",
        "入群时间",
    ])
    .wrap_err("Failed to write csv header")?;

    for (i, member) in table.iter().enumerate() {
        wtr.write_record(&[
            // &i.to_string(),
            &member.qq_name,
            &member.group_name,
            &member.qq_name,
            &member.gender.to_string(),
            &member.qq_age.to_string(),
            &member.joined_date.to_string(),
        ])
        .wrap_err_with(|| format!("Filed to write record {member:?}"))?;
    }
    wtr.flush()
        .wrap_err_with(|| format!("Failed to flush csv writer for {out_path:?}"))?;
    Ok(())
}
