use cf_kv::{
    capnproto::{CapnprotoClient, CapnprotoKVServer},
    cornflakes_dynamic::{CornflakesClient, CornflakesKVServer},
    flatbuffers::{FlatbuffersClient, FlatbuffersKVServer},
    protobuf::{ProtobufClient, ProtobufKVServer},
    retwis::{retwis_keys, RetwisClient, RetwisServerLoader},
    retwis_run_datapath::*,
    run_client_retwis, run_mlx5_cornflakes_with_zcc, run_server_retwis,
    set_zcc_and_mempool_parameters, KVClient,
};
use color_eyre::eyre::Result;
use cornflakes_libos::{
    datapath::Datapath, state_machine::client::ClientSM, state_machine::server::ServerSM,
};
use cornflakes_utils::{global_debug_init, AppMode, SerializationType};
use mlx5_datapath::datapath::connection::{CornflakesMlx5Slab, Mlx5Connection};
use structopt::StructOpt;

fn main() -> Result<()> {
    let mut opt = RetwisOpt::from_args();
    global_debug_init(opt.trace_level)?;
    check_opt(&mut opt)?;

    match opt.mode {
        AppMode::Server => match opt.serialization {
            SerializationType::CornflakesDynamic | SerializationType::CornflakesOneCopyDynamic => {
                run_mlx5_cornflakes_with_zcc!(opt, run_server_retwis);
            }
            SerializationType::Flatbuffers => {
                run_server_retwis!(
                    FlatbuffersKVServer<
                        Mlx5Connection<
                            zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                        >,
                    >,
                    Mlx5Connection<
                        zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                    >,
                    opt
                );
            }
            SerializationType::Capnproto => {
                run_server_retwis!(
                    CapnprotoKVServer<
                        Mlx5Connection<
                            zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                        >,
                    >,
                    Mlx5Connection<
                        zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                    >,
                    opt
                );
            }
            SerializationType::Protobuf => {
                run_server_retwis!(
                    ProtobufKVServer<
                        Mlx5Connection<
                            zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                        >,
                    >,
                    Mlx5Connection<
                        zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                    >,
                    opt
                );
            }
            _ => {
                unimplemented!();
            }
        },
        AppMode::Client => match opt.serialization {
            SerializationType::CornflakesDynamic | SerializationType::CornflakesOneCopyDynamic => {
                run_client_retwis!(
                    CornflakesClient<
                        Mlx5Connection<
                            zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                        >,
                    >,
                    Mlx5Connection<
                        zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                    >,
                    opt
                );
            }
            SerializationType::Flatbuffers => {
                run_client_retwis!(
                    FlatbuffersClient<
                        Mlx5Connection<
                            zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                        >,
                    >,
                    Mlx5Connection<
                        zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                    >,
                    opt
                );
            }
            SerializationType::Capnproto => {
                run_client_retwis!(
                    CapnprotoClient<
                        Mlx5Connection<
                            zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                        >,
                    >,
                    Mlx5Connection<
                        zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                    >,
                    opt
                );
            }
            SerializationType::Protobuf => {
                run_client_retwis!(
                    ProtobufClient<
                        Mlx5Connection<
                            zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                        >,
                    >,
                    Mlx5Connection<
                        zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>,
                    >,
                    opt
                );
            }
            _ => {
                unimplemented!();
            }
        },
    }
    Ok(())
}
