mod reader;
mod remote_file;
mod types;

use std::fmt::Debug;

use remote_file::RemoteFile;
use types::{zip, FileType};

use structopt::StructOpt;
use ureq::Proxy;
use url::Url;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    /// The remote url for the target file
    #[structopt(short, long)]
    url: String,

    /// Use the specified proxy
    #[structopt(short, long)]
    proxy: Option<String>,

    /// Use the specified offset
    #[structopt(short, long, default_value = "0")]
    offset: usize,
}

fn decide_archive(remote_file: RemoteFile) -> impl Iterator<Item = impl Debug> + FileType {
    match remote_file.content_type() {
        remote_file::SupportedTypes::Zip => zip::ZipFile::new(remote_file),
        remote_file::SupportedTypes::Unsupported => todo!(),
    }
}
fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();

    let _ = Url::parse(&opt.url).expect("the provided url should be a valid url");

    let mut agent_builder = ureq::AgentBuilder::new();

    if let Some(opt_proxy) = opt.proxy {
        let proxy =
            Proxy::new(opt_proxy).expect("the provided poxy url should be a valid proxy url");
        agent_builder = agent_builder.proxy(proxy);
    }

    let client = agent_builder.build();
    let remote_file = RemoteFile::try_new(client, &opt.url)
        .expect("the target server is not reachable or it is not supporting the range header");

    let mut archive = decide_archive(remote_file);

    if opt.offset != 0 {
        let _start_from = archive.start_from(opt.offset)?;
    }

    for entry in archive {
        dbg!(&entry);
    }

    Ok(())
}
