use std::borrow::Cow;
use std::convert::Infallible;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

use binrw::BinReaderExt;
use disc_riider::builder::{
    DirPartitionBuilder, PartitionAddError, WiiDiscBuilder, WiiPartitionDefinition,
};
use disc_riider::dir_reader::{build_fst_from_directory_tree, BuildDirError};
use disc_riider::structs::{Certificate, DiscHeader, Ticket, WiiPartType, TMD};
use disc_riider::{Fst, FstNode};

#[derive(thiserror::Error, Debug)]
pub enum IsoBuildErr {
    #[error("{0}")]
    BuildDirError(#[from] BuildDirError),
    #[error("{0:?}:{1:?}")]
    FileOpenErr(PathBuf, io::Error),
}

pub struct SSRandoIsoBuilder {
    base_dir: PathBuf,
    fst: Fst,
    buf: Vec<u8>,
}

type DirPartAddErr = PartitionAddError<IsoBuildErr>;

fn try_open(path: PathBuf) -> Result<binrw::io::BufReader<File>, DirPartAddErr> {
    File::open(&path)
        .map_err(|e| PartitionAddError::Custom(IsoBuildErr::FileOpenErr(path, e)))
        .map(|f| binrw::io::BufReader::new(f))
}

impl WiiPartitionDefinition<IsoBuildErr> for SSRandoIsoBuilder {
    fn get_disc_header(&mut self) -> Result<DiscHeader, DirPartAddErr> {
        let mut path = self.base_dir.clone();
        path.push("sys");
        path.push("boot.bin");
        let header = try_open(path)?.read_be::<DiscHeader>()?;
        Ok(header)
    }

    fn get_bi2<'a>(&'a mut self) -> Result<Cow<'a, [u8]>, DirPartAddErr> {
        let mut path = self.base_dir.clone();
        path.push("sys");
        path.push("bi2.bin");
        let mut f = try_open(path)?;
        self.buf.clear();
        f.read_to_end(&mut self.buf)?;
        Ok(Cow::Borrowed(&self.buf))
    }

    fn get_apploader<'a>(&'a mut self) -> Result<Cow<'a, [u8]>, DirPartAddErr> {
        self.buf.clear();
        let mut path = self.base_dir.clone();
        path.push("sys");
        path.push("apploader.img");
        let mut f = try_open(path)?;
        f.read_to_end(&mut self.buf)?;
        Ok(Cow::Borrowed(&self.buf))
    }

    fn get_fst(&mut self) -> Result<Fst, DirPartAddErr> {
        Ok(self.fst.clone())
    }

    fn get_dol<'a>(&'a mut self) -> Result<Cow<'a, [u8]>, DirPartAddErr> {
        self.buf.clear();
        let mut path = self.base_dir.clone();
        path.push("sys");
        path.push("main.dol");
        let mut f = try_open(path)?;
        f.read_to_end(&mut self.buf)?;
        Ok(Cow::Borrowed(&self.buf))
    }

    fn get_file_data<'a>(
        &'a mut self,
        path: &Vec<String>,
    ) -> Result<(Cow<'a, [u8]>, u32), DirPartAddErr> {
        let mut fs_path = self.base_dir.clone();
        fs_path.push("files");
        for part in path.iter() {
            fs_path.push(part);
        }
        self.buf.clear();
        let mut f = try_open(fs_path)?;
        f.read_to_end(&mut self.buf)?;
        Ok((Cow::Borrowed(&self.buf), 0))
    }

    fn progress_callback(&mut self, processed_files: usize, total_files: usize) {
        println!(
            "[{}/{} {}%]",
            processed_files,
            total_files,
            (processed_files as f32 / total_files as f32 * 100f32) as u8
        );
    }
}

pub fn build_from_directory<WS: Write + Seek + Read>(
    dir: &Path,
    dest: &mut WS,
) -> Result<(), DirPartAddErr> {
    let mut disc_header = {
        let mut path = dir.to_owned();
        path.push("DATA");
        path.push("sys");
        path.push("boot.bin");
        try_open(path)?.read_be::<DiscHeader>()?
    };
    disc_header.disable_disc_enc = 0;
    disc_header.disable_hash_verification = 0;
    let region = {
        let mut path = dir.to_owned();
        path.push("DATA");
        path.push("disc");
        path.push("region.bin");
        let mut f = try_open(path)?;
        let mut region = [0; 32];
        f.read_exact(&mut region)?;
        region
    };
    let mut builder = WiiDiscBuilder::create(dest, disc_header, region);
    let mut partition_path = dir.to_owned();
    partition_path.push("DATA");
    let ticket = {
        let mut path = partition_path.clone();
        path.push("ticket.bin");
        let mut f = try_open(path)?;
        f.read_be::<Ticket>()?
    };
    let tmd = {
        let mut path = partition_path.clone();
        path.push("tmd.bin");
        let mut f = try_open(path)?;
        f.read_be::<TMD>()?
    };
    let cert_chain = {
        let mut path = partition_path.clone();
        path.push("cert.bin");
        let mut f = try_open(path)?;
        f.read_be::<[Certificate; 3]>()?
    };
    let mut files_dir = partition_path.clone();
    files_dir.push("files");
    let fst = build_fst_from_directory_tree(&files_dir)
        .map_err(|e| PartitionAddError::Custom(e.into()))?;
    let mut total_file_count = 0;
    fst.callback_all_files::<Infallible, _>(&mut |_, node| {
        if matches!(node, FstNode::File { .. }) {
            total_file_count += 1;
        }
        Ok(())
    })
    .unwrap();
    let mut dir_builder = SSRandoIsoBuilder {
        base_dir: partition_path,
        buf: Vec::new(),
        fst,
    };
    builder.add_partition(WiiPartType::Data, ticket, tmd, cert_chain, &mut dir_builder)?;
    builder.finish()?;
    Ok(())
}
