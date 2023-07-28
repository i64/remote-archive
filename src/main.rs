mod adapter;
mod file_types;
mod utils;

use argh::FromArgs;
use exact_reader::{ExactReader, MultiFile};
use file_types::FileType;
use url::Url;
use utils::{build_tree, display_tree};

use adapter::RemoteAdapter;
use ureq::Proxy;

const HELP: &str = concat!(
    "Usage: {remote_archive} [--url <url>] [-f <file>] [-p <proxy>] [-t]\n\n",
    "Options:\n",
    "  --url             the URL to download(mutually exclusive with 'file')\n",
    "  -f, --file        the file to process (mutually exclusive with 'url')\n",
    "  -p, --proxy       the proxy server to use\n",
    "  -t, --tree        to show file contents as tree\n",
    "  --help            display usage information"
);

pub fn new_multi<S: AsRef<str>>(
    client: &ureq::Agent,
    urls: Vec<S>,
) -> ExactReader<MultiFile<RemoteAdapter>> {
    let files = urls
        .iter()
        .map(|url| dbg!(url.as_ref()))
        .map(|url| RemoteAdapter::try_new(client.clone(), url).unwrap().into())
        .collect();
    let multifile = MultiFile::new(files);

    ExactReader::new_multi(multifile)
}
#[derive(FromArgs)]
/// A utility for exploring remote archive files without downloading the entire contents of the archive
struct Arguments {
    #[argh(option, long = "url")]
    /// the URL to download (mutually exclusive with 'file')
    url: Option<String>,

    #[argh(option, long = "file", short = 'f')]
    /// the file to process (mutually exclusive with 'url')
    file: Option<String>,

    #[argh(option, long = "proxy", short = 'p')]
    /// the proxy server to use
    proxy: Option<String>,

    #[argh(switch, long = "tree", short = 't')]
    /// to show file contents as tree
    tree: bool,
}
fn main() -> std::io::Result<()> {
    let opt: Arguments = argh::from_env();

    if opt.file.is_some() && opt.url.is_some() {
        eprintln!("Error: Options --file and --url are mutually exclusive.");
        eprintln!("{HELP}");
        std::process::exit(1);
    }

    let mut agent_builder = ureq::AgentBuilder::new();

    if let Some(opt_proxy) = opt.proxy {
        let proxy =
            Proxy::new(opt_proxy).expect("the provided poxy url should be a valid proxy url");
        agent_builder = agent_builder.proxy(proxy);
    }

    let client = agent_builder.build();

    let reader = {
        if let Some(url) = opt.url {
            let _ = Url::parse(&url).expect("the provided url should be a valid url");
            new_multi(&client, vec![url])
        } else if let Some(filename) = opt.file {
            let file_data = std::fs::read(filename)?;
            let urls: Vec<_> = String::from_utf8_lossy(&file_data)
                .lines()
                .map(|l| l.to_owned())
                .collect();

            new_multi(&client, urls)
        } else {
            eprintln!("{HELP}");
            std::process::exit(1);
        }
    };

    let mut zip = file_types::zip::ZipFile::new(reader);
    let paths_iter = zip.entry_iter()?.map(|e| e.filename);
    if opt.tree {
        let paths = paths_iter.collect();
        let tree = build_tree(paths);
        display_tree(&tree, 0);
    } else {
        paths_iter.for_each(|p| println!("{p}"))
    }
    Ok(())
}