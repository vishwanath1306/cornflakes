use super::retwis::{RetwisRequestDistribution, RetwisValueSizeGenerator};
use color_eyre::eyre::{bail, Result};
use cornflakes_libos::{
    datapath::{InlineMode, PushBufType},
    loadgen::request_schedule::DistributionType,
};
use cornflakes_utils::{AppMode, SerializationType, TraceLevel};
use std::net::Ipv4Addr;
use structopt::StructOpt;

#[macro_export]
macro_rules! run_server_retwis(
    ($kv_server: ty, $datapath: ty, $opt: ident) => {
        let is_baseline = is_baseline(&$opt);
        cornflakes_libos::datapath::set_mempool_params($opt.num_pages_per_mempool, $opt.num_registrations, !$opt.do_not_register_at_start);
        let mut datapath_params = <$datapath as Datapath>::parse_config_file(&$opt.config_file, &$opt.server_ip)?;
        let addresses = <$datapath as Datapath>::compute_affinity(&datapath_params, 1, None, AppMode::Server)?;
        let per_thread_contexts = <$datapath as Datapath>::global_init(1, &mut datapath_params, addresses)?;
        let mut connection = <$datapath as Datapath>::per_thread_init(datapath_params, per_thread_contexts.into_iter().nth(0).unwrap(),
        AppMode::Server)?;

        connection.set_copying_threshold($opt.copying_threshold);
        connection.set_inline_mode($opt.inline_mode);
        tracing::info!(threshold = $opt.copying_threshold, "Setting zero-copy copying threshold");

        // init retwis load generator
        let load_generator = RetwisServerLoader::new($opt.num_keys, $opt.key_size, $opt.value_size_generator);
        let mut kv_server = <$kv_server>::new("", load_generator, &mut connection, $opt.push_buf_type, false)?;
        kv_server.init(&mut connection)?;
        kv_server.write_ready($opt.ready_file.clone())?;
        if is_baseline {
            kv_server.run_state_machine_baseline(&mut connection)?;
        } else {
            kv_server.run_state_machine(&mut connection)?;
        }
    }
);

#[macro_export]
macro_rules! run_client_retwis(
    ($serializer: ty, $datapath: ty, $opt: ident) => {
        let server_addr = cornflakes_utils::parse_server_addr(&$opt.config_file, &$opt.server_ip)?;
        let mut datapath_params = <$datapath as Datapath>::parse_config_file(&$opt.config_file, &$opt.our_ip)?;
        let addresses = <$datapath as Datapath>::compute_affinity(
                &datapath_params,
                $opt.num_threads,
                Some($opt.server_ip.clone()),
                cornflakes_utils::AppMode::Client,
            )?;
        let num_rtts = ($opt.rate * $opt.total_time * 2) as usize;
        let schedules =
            cornflakes_libos::loadgen::request_schedule::generate_schedules(num_rtts, $opt.rate as _, $opt.distribution, $opt.num_threads)?;

        let per_thread_contexts = <$datapath as Datapath>::global_init(
            $opt.num_threads,
            &mut datapath_params,
            addresses,
        )?;
        let mut threads: Vec<std::thread::JoinHandle<Result<cornflakes_libos::loadgen::client_threads::ThreadStats>>> = vec![];

        let mut retwis_keys = retwis_keys($opt.num_keys, $opt.key_size);
        // spawn a thread to run client for each connection
        for (i, (schedule, per_thread_context)) in schedules
            .into_iter()
            .zip(per_thread_contexts.into_iter())
            .enumerate()
        {
        let thread_keys = retwis_keys.clone();
        let server_addr_clone =
            cornflakes_libos::utils::AddressInfo::new(server_addr.2, server_addr.1.clone(), server_addr.0.clone());
            let datapath_params_clone = datapath_params.clone();

            let max_num_requests = num_rtts;
            let opt_clone = $opt.clone();
            threads.push(std::thread::spawn(move || {
                match affinity::set_thread_affinity(&vec![i + 1]) {
                    Ok(_) => {}
                    Err(e) => {
                        color_eyre::eyre::bail!(
                            "Could not set thread affinity for thread {} on core {}: {:?}",
                            i,
                            i + 1,
                            e
                         )
                    }
                }

                let mut connection = <$datapath as Datapath>::per_thread_init(
                    datapath_params_clone,
                    per_thread_context,
                    cornflakes_utils::AppMode::Client,
                )?;

                connection.set_copying_threshold(std::usize::MAX);

                tracing::info!("Finished initializing datapath connection for thread {}", i);
                let size = opt_clone.value_size_generator.avg_size();
                let mut retwis_client = RetwisClient::new(thread_keys, opt_clone.zipf, opt_clone.value_size_generator, opt_clone.retwis_distribution)?;
                tracing::info!("Finished initializing retwis client");

                let mut server_load_generator_opt: Option<(&str, RetwisServerLoader)> = None;
                let mut kv_client: KVClient<RetwisClient, $serializer, $datapath> = KVClient::new(retwis_client, server_addr_clone, max_num_requests,opt_clone.retries, server_load_generator_opt)?;


                kv_client.init(&mut connection)?;

                cornflakes_libos::state_machine::client::run_client_loadgen(i, opt_clone.num_threads as _, opt_clone.client_id as _, opt_clone.num_clients as _, &mut kv_client, &mut connection, opt_clone.retries, opt_clone.total_time as _, opt_clone.logfile.clone(), opt_clone.rate as _, size as _, schedule, opt_clone.ready_file.clone())
            }));
        }

        let mut thread_results: Vec<cornflakes_libos::loadgen::client_threads::ThreadStats> = Vec::default();
        for child in threads {
            let s = match child.join() {
                Ok(res) => match res {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("Thread failed: {:?}", e);
                        color_eyre::eyre::bail!("Failed thread");
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to join client thread: {:?}", e);
                    color_eyre::eyre::bail!("Failed to join thread");
                }
            };
            thread_results.push(s);
        }

        let dump_per_thread = $opt.logfile == None;
        cornflakes_libos::loadgen::client_threads::dump_thread_stats(thread_results, $opt.thread_log.clone(), dump_per_thread)?;
    }
);

fn is_cf(opt: &RetwisOpt) -> bool {
    opt.serialization == SerializationType::CornflakesDynamic
        || opt.serialization == SerializationType::CornflakesOneCopyDynamic
}

pub fn is_baseline(opt: &RetwisOpt) -> bool {
    !(opt.serialization == SerializationType::CornflakesOneCopyDynamic
        || opt.serialization == SerializationType::CornflakesDynamic)
}

pub fn check_opt(opt: &mut RetwisOpt) -> Result<()> {
    if !is_cf(opt) && opt.push_buf_type != PushBufType::SingleBuf {
        bail!("For non-cornflakes serialization, push buf type must be single buffer.");
    }

    if opt.serialization == SerializationType::CornflakesOneCopyDynamic {
        // copy all segments
        opt.copying_threshold = usize::MAX;
    }

    Ok(())
}
#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "Retwis KV Store App.",
    about = "Retwis KV store server and client."
)]
pub struct RetwisOpt {
    #[structopt(
        short = "debug",
        long = "debug_level",
        help = "Configure tracing settings.",
        default_value = "warn"
    )]
    pub trace_level: TraceLevel,
    #[structopt(
        short = "cf",
        long = "config_file",
        help = "Folder containing shared config information."
    )]
    pub config_file: String,
    #[structopt(long = "mode", help = "KV server or client mode.")]
    pub mode: AppMode,
    #[structopt(long = "time", help = "max time to run exp for", default_value = "30")]
    pub total_time: usize,
    #[structopt(
        long = "push_buf_type",
        help = "Push API to use",
        default_value = "sga"
    )]
    pub push_buf_type: PushBufType,
    #[structopt(
        long = "inline_mode",
        help = "For Mlx5 datapath, which inline mode to use. Note this can't be set for DPDK datapath.",
        default_value = "nothing"
    )]
    pub inline_mode: InlineMode,
    #[structopt(
        long = "copy_threshold",
        help = "Datapath copy threshold. Copies everything below this threshold. If set to 0, tries to use zero-copy for everything. If set to infinity, uses zero-copy for nothing.",
        default_value = "256"
    )]
    pub copying_threshold: usize,
    #[structopt(
        short = "r",
        long = "rate",
        help = "Rate of client (in pkts/sec)",
        default_value = "2000"
    )]
    pub rate: usize,
    #[structopt(
        long = "server_ip",
        help = "Server ip address",
        default_value = "127.0.0.1"
    )]
    pub server_ip: Ipv4Addr,
    #[structopt(long = "our_ip", help = "Our ip address", default_value = "127.0.0.1")]
    pub our_ip: Ipv4Addr,
    #[structopt(
        long = "serialization",
        help = "Serialization library to use",
        default_value = "cornflakes-dynamic"
    )]
    pub serialization: SerializationType,
    #[structopt(long = "retries", help = "Enable client retries.")]
    pub retries: bool,
    #[structopt(long = "logfile", help = "Logfile to log all client RTTs.")]
    pub logfile: Option<String>,
    #[structopt(long = "threadlog", help = "Logfile to log per thread statistics")]
    pub thread_log: Option<String>,
    #[structopt(
        long = "num_threads",
        help = "Number of (client) threads.",
        default_value = "1"
    )]
    pub num_threads: usize,
    #[structopt(
        long = "num_clients",
        help = "Total number of clients",
        default_value = "1"
    )]
    pub num_clients: usize,
    #[structopt(long = "client_id", default_value = "0")]
    pub client_id: usize,
    #[structopt(long = "start_cutoff", default_value = "0")]
    pub start_cutoff: usize,
    #[structopt(long = "distribution", default_value = "exponential")]
    pub distribution: DistributionType,
    #[structopt(
        long = "num_keys",
        default_value = "1000000",
        help = "Default number of keys to initialize the KV store with"
    )]
    pub num_keys: usize,
    #[structopt(long = "key_size", default_value = "64", help = "Default key size")]
    pub key_size: usize,
    #[structopt(long = "value_distribution", default_value = "SingleValue-1024")]
    pub value_size_generator: RetwisValueSizeGenerator,
    #[structopt(
        long = "zipf",
        default_value = "0.75",
        help = "Zipf distribution to  choose keys from"
    )]
    pub zipf: f64,
    #[structopt(
        long = "retwis_distribution",
        help = "Request Distribution for Retwis",
        default_value = "5-15-30-50"
    )]
    pub retwis_distribution: RetwisRequestDistribution,
    #[structopt(
        long = "ready_file",
        help = "File to indicate server is ready to receive requests"
    )]
    pub ready_file: Option<String>,
    #[structopt(
        long = "num_pages",
        help = "Number of pages per allocated mempool",
        default_value = "64"
    )]
    pub num_pages_per_mempool: usize,
    #[structopt(
        long = "num_registrations",
        help = "Number of registrations per allocated mempool",
        default_value = "1"
    )]
    pub num_registrations: usize,
    #[structopt(
        long = "dont_register_at_start",
        help = "Register mempool memory at start"
    )]
    pub do_not_register_at_start: bool,
}
