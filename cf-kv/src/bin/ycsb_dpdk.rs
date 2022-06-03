use cf_kv::{
    cornflakes_dynamic::{CornflakesClient, CornflakesKVServer},
    run_client, run_server,
    ycsb::{YCSBClient, YCSBServerLoader},
    ycsb_run_datapath::*,
    KVClient, RequestGenerator,
};
use color_eyre::eyre::Result;
use cornflakes_libos::{
    datapath::Datapath, state_machine::client::ClientSM, state_machine::server::ServerSM,
};
use cornflakes_utils::{global_debug_init, AppMode, SerializationType};
use dpdk_datapath::datapath::connection::DpdkConnection;
use structopt::StructOpt;

fn main() -> Result<()> {
    let mut opt = YCSBOpt::from_args();
    global_debug_init(opt.trace_level)?;
    check_opt(&mut opt)?;

    match opt.mode {
        AppMode::Server => match opt.serialization {
            SerializationType::CornflakesDynamic | SerializationType::CornflakesOneCopyDynamic => {
                run_server!(CornflakesKVServer<DpdkConnection>, DpdkConnection, opt);
            }
            _ => {
                unimplemented!();
            }
        },
        AppMode::Client => match opt.serialization {
            SerializationType::CornflakesDynamic | SerializationType::CornflakesOneCopyDynamic => {
                run_client!(CornflakesClient<DpdkConnection>, DpdkConnection, opt);
            }
            _ => {
                unimplemented!();
            }
        },
    }
    Ok(())
}
