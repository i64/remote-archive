mod reader;
mod remote_file;
mod types;

use futures::{
    future,
    stream::{self, StreamExt},
};
use std::{fmt::Debug, sync::Arc};

use remote_file::RemoteFile;
use types::{zip, FileType};

// use structopt::StructOpt;
use argh::FromArgs;

const STEP_SIZE: usize = {
    let K = 1024usize;
    let one_byte = 1;
    let one_kb = K * one_byte;
    let one_mb = K * one_kb;
    let one_gb = K * one_mb;
    5 * one_gb
};

#[derive(FromArgs, Debug)]
/// ...
struct Args {
    /// the remote url for the target file
    #[argh(option, short = 'u')]
    url: String,

    /// use the specified proxy
    #[argh(option, short = 'p')]
    proxy: Option<String>,

    /// use the specified offset
    #[argh(option, short = 'o', default = "0")]
    offset: usize,

    /// workers count
    #[argh(option, short = 'w', default = "10")]
    workers: usize,

    /// chunk size for a single worker
    #[argh(option, default = "STEP_SIZE")]
    worker_chunk_size: usize,

    /// haystack size for the offset based search
    #[argh(option, default = "4096")]
    haystack_size: usize,
}
// TODO: ALIGN MAP
async fn task_executer<A>(archive: &mut A, offset: usize, chunk_size: usize, haystack_size: usize)
where
    A: FileType,
    types::Entry<<A as types::FileType>::EntryType>: Debug,
{
    dbg!(offset);
    if matches!(archive.start_from(offset, haystack_size).await, Err(_)) {
        return;
    }

    while let Ok(entry) = archive.read_entry().await {
        println!("yielding from {offset}, {}", entry.range.end);
        dbg!(&entry);

        if entry.range.end >= (offset + chunk_size) as u64 {
            break;
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let opt: Args = argh::from_env();

    let mut client_builder = reqwest::ClientBuilder::new();

    if let Some(opt_proxy) = opt.proxy {
        let proxy = reqwest::Proxy::all(opt_proxy)
            .expect("the provided poxy url should be a valid proxy url");
        client_builder = client_builder.proxy(proxy);
    }

    let client = client_builder.build().unwrap();
    let remote_file = RemoteFile::try_new(client, &opt.url)
        .await
        .expect("the target server is not reachable or it is not supporting the range header");

    let size = remote_file.size;
    let mut archive = zip::ZipFile::new(remote_file);

    let fut = stream::repeat(archive)
        .zip(stream::iter(
            (opt.offset..size).step_by(opt.worker_chunk_size),
        ))
        .for_each_concurrent(opt.workers, |(mut archive, offset)| async move {
            task_executer(
                &mut archive,
                offset,
                opt.worker_chunk_size,
                opt.haystack_size,
            )
            .await;
        });

    fut.await;

    Ok(())
}
