use cf_kv::{
    capnproto::{CapnprotoClient, CapnprotoKVServer},
    cornflakes_dynamic::{CornflakesClient, CornflakesKVServer},
    flatbuffers::{FlatbuffersClient, FlatbuffersKVServer},
    init_zcc_pin_and_unpin_thread,
    protobuf::{ProtobufClient, ProtobufKVServer},
    redis::RedisClient,
    run_client, run_mlx5_cornflakes_with_zcc, run_server, set_zcc_and_mempool_parameters,
    ycsb::{YCSBClient, YCSBServerLoader},
    ycsb_run_datapath::*,
    KVClient,
};
use color_eyre::eyre::Result;
use cornflakes_libos::{
    datapath::Datapath, state_machine::client::ClientSM, state_machine::server::ServerSM,
};
use cornflakes_utils::{global_debug_init, AppMode, SerializationType};
use mlx5_datapath::datapath::connection::{CornflakesMlx5Slab, Mlx5Connection};
use structopt::StructOpt;
use zero_copy_cache;

fn main() -> Result<()> {
    let mut opt = YCSBOpt::from_args();
    global_debug_init(opt.trace_level)?;
    check_opt(&mut opt)?;

    match opt.mode {
        AppMode::Server => match opt.serialization {
            SerializationType::CornflakesDynamic | SerializationType::CornflakesOneCopyDynamic => {
                run_mlx5_cornflakes_with_zcc!(opt, run_server);
            }
            SerializationType::Flatbuffers => {
                run_server!(
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
                run_server!(
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
                run_server!(
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
                run_client!(
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
                run_client!(
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
                run_client!(
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
            SerializationType::Redis => {
                run_client!(
                    RedisClient<
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
                run_client!(
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
