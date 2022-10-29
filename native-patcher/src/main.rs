use std::{
    convert::Infallible,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{anyhow, bail};
use binrw::{BinReaderExt, BinWriterExt};
use builder::build_from_directory;
use clap::Parser;
use disc_riider::{
    structs::{DiscHeader, WiiPartTableEntry, WiiPartType, WiiPartitionHeader},
    Fst, FstNode, WiiIsoReader,
};
use sha1::digest::{FixedOutputReset, Update};

mod builder;

const CORRECT_DOL_HASH: [u8; 20] = [
    69, 10, 104, 6, 244, 109, 89, 220, 248, 39, 141, 176, 142, 6, 249, 72, 101, 164, 177, 138,
];

#[derive(Debug, Parser)]
/// Tool to perform the "hard" work for the skyward sword randomizer
enum Commands {
    /// extract the entire ISO into a folder, doing an itegrity check and not
    /// extracting the hint movies
    Extract {
        filename: PathBuf,
        destination: PathBuf,
    },
    /// build an ISO from an extract, potentially
    Rebuild {
        src_dir: PathBuf,
        dest_file: PathBuf,
    },
}

fn create_file(p: &Path, parts: &[&str]) -> io::Result<File> {
    let mut path = p.to_path_buf();
    path.extend(parts);
    fs::create_dir_all(path.parent().unwrap())?;
    File::create(path)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let command = Commands::parse();
    match command {
        Commands::Extract {
            filename,
            destination,
        } => {
            log::info!("extracting from {filename:?} to {destination:?}");
            let f = File::open(filename)?;
            let mut reader = WiiIsoReader::create(f)?;
            // we only care about the DATA section
            let part_idx = reader
                .partitions()
                .iter()
                .position(|p| p.part_type == WiiPartType::Data)
                .ok_or_else(|| anyhow!("ISO does not have a DATA section"))?;
            let mut files_destination = destination;
            files_destination.push("DATA");

            // write_to_path(&files_destination, &["sys", "boot.bin"], reader.get_header())?;
            let region_blob = reader.get_region().clone();

            // temporarily open the partition to check the dol
            let mut part_reader = reader.open_partition_stream_by_index(part_idx)?;
            let mut crypt_reader = part_reader.open_encryption_reader();
            let disc_header: DiscHeader = crypt_reader.read_be()?;
            let dol = crypt_reader.read_dol(*disc_header.dol_off)?;

            // hash dol
            let mut hasher = sha1::Sha1::default();
            hasher.update(&dol);
            let dol_hash = hasher.finalize_fixed_reset();

            if dol_hash.as_slice() != CORRECT_DOL_HASH {
                let mut hash_str = String::with_capacity(40);
                // "pretty print" the hash
                for byte in dol_hash {
                    use std::fmt::Write;
                    // this should never return an error anyways, but
                    // panicking here seems more silly
                    let _ = write!(&mut hash_str, "{:02x}", byte);
                }
                bail!("wrong dol checksum: {hash_str}!");
            }

            // TODO: actually check hash and error

            drop(crypt_reader);
            drop(part_reader);

            // now we can actually start writing files

            create_file(&files_destination, &["disc", "region.bin"])?.write_all(&region_blob)?;

            let mut part_reader = reader.open_partition_stream_by_index(part_idx)?;
            let part_header = part_reader.get_header();

            create_file(&files_destination, &["ticket.bin"])?.write_be(&part_header.ticket)?;
            create_file(&files_destination, &["tmd.bin"])?.write_be(&part_reader.read_tmd()?)?;
            create_file(&files_destination, &["cert.bin"])?
                .write_be(&part_reader.read_certificates()?)?;

            let mut crypt_reader = part_reader.open_encryption_reader();

            crypt_reader.seek(SeekFrom::Start(0))?;
            let disc_header: DiscHeader = crypt_reader.read_be()?;
            create_file(&files_destination, &["sys", "boot.bin"])?.write_be(&disc_header)?;
            let mut bi2 = vec![0; 0x2000];
            crypt_reader.read_exact(&mut bi2)?;
            create_file(&files_destination, &["sys", "bi2.bin"])?.write_all(&bi2)?;
            let apploader = crypt_reader.read_apploader()?;
            create_file(&files_destination, &["sys", "apploader.img"])?.write_be(&apploader)?;

            create_file(&files_destination, &["sys", "main.dol"])?.write_all(&dol)?;
            let mut fst_buf = Vec::new();
            crypt_reader.read_into_vec(
                *disc_header.fst_off,
                *disc_header.fst_sz as u64,
                &mut fst_buf,
            )?;
            create_file(&files_destination, &["sys", "fst.bin"])?.write_all(&fst_buf)?;

            let mut fst = Fst::read(&mut crypt_reader, *disc_header.fst_off)?;

            // we don't want to extract hintmovies
            let thp_dir = fst.find_node_path_mut("THP");
            match thp_dir {
                Some(FstNode::Directory { files, .. }) => {
                    files.retain(|f| f.get_name().contains("Demo"));
                }
                _ => bail!("can't find THP directory in the ISO!"),
            }

            let mut buf = Vec::new();
            files_destination.push("files");
            let mut total_filecount = 0;
            fst.callback_all_files::<Infallible, _>(&mut |_, node| {
                if matches!(node, FstNode::File { .. }) {
                    total_filecount += 1;
                }
                Ok(())
            })
            // TODO: use into_ok once it's stable
            .unwrap();
            let mut current_filecount = 0;
            fst.callback_all_files::<io::Error, _>(&mut |path, node| {
                if let &FstNode::File { offset, length, .. } = node {
                    let mut out_path = files_destination.clone();
                    for part in path {
                        out_path.push(part);
                    }
                    log::info!("extracting: {}", node.get_name());
                    println!(
                        "[{}/{} {}%]",
                        current_filecount,
                        total_filecount,
                        (current_filecount as f32 / total_filecount as f32 * 100f32) as u8
                    );
                    fs::create_dir_all(out_path.parent().unwrap())?;
                    let mut outf = File::create(out_path)?;
                    let start = Instant::now();
                    crypt_reader.read_into_vec(offset, length as u64, &mut buf)?;
                    log::debug!("reading: {}", start.elapsed().as_millis());
                    let start = Instant::now();
                    outf.write_all(&buf)?;
                    drop(outf);
                    log::debug!("writing: {}", start.elapsed().as_millis());
                    current_filecount += 1;
                }
                Ok(())
            })?;
        }
        Commands::Rebuild { src_dir, dest_file } => {
            let mut outf = OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(dest_file)?;
            build_from_directory(&src_dir, &mut outf)?;
        }
    }
    Ok(())
}
