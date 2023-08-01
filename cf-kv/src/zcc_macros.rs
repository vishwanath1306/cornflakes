#[macro_export]
macro_rules! init_zcc_logging(
    ($opt: ident, $server: ident, $conn: ident) => {
        if let Some(_f) = $opt.record_pinning_map {
            // TODO: configure ZCC to record the pinning map on the other thread
        }
    }

);
#[macro_export]
macro_rules! init_zcc_pin_and_unpin_thread(
    ($opt: ident, $conn: ident) => {
        if $opt.zcc_alg != zero_copy_cache::data_structures::CacheType::NoAlg  {
            if !$opt.zcc_pin_on_demand {
                $conn.initialize_zero_copy_cache_thread()?;
            }
        }
    }
);
#[macro_export]
macro_rules! set_zcc_and_mempool_parameters(
    ($opt: ident) => {
        cornflakes_libos::datapath::set_mempool_params($opt.num_pages_per_mempool, !$opt.do_not_register_at_start)?;
       cornflakes_libos::datapath::set_zcc_params($opt.zcc_pinning_limit_2mb_pages, $opt.zcc_segment_size_2mb_pages, $opt.zcc_alg, $opt.zcc_pin_on_demand, $opt.zcc_sleep_duration, $opt.record_pinning_map.clone())?;
    }
);
#[macro_export]
macro_rules! run_mlx5_cornflakes_with_zcc(
    ($opt: ident, $method: ident) => {
        match $opt.zcc_alg {
            zero_copy_cache::data_structures::CacheType::OnDemandLru => {
                $method!(CornflakesKVServer<Mlx5Connection<zero_copy_cache::data_structures::OnDemandLruCache<CornflakesMlx5Slab>>>, Mlx5Connection<zero_copy_cache::data_structures::OnDemandLruCache<CornflakesMlx5Slab>>, $opt);
            }
            zero_copy_cache::data_structures::CacheType::TimestampLru => {
                $method!(CornflakesKVServer<Mlx5Connection<zero_copy_cache::data_structures::TimestampLruCache<CornflakesMlx5Slab>>>, Mlx5Connection<zero_copy_cache::data_structures::TimestampLruCache<CornflakesMlx5Slab>>, $opt);
            }
            zero_copy_cache::data_structures::CacheType::LinkedListLru => {
                $method!(CornflakesKVServer<Mlx5Connection<zero_copy_cache::data_structures::LinkedListLruCache<CornflakesMlx5Slab>>>, Mlx5Connection<zero_copy_cache::data_structures::LinkedListLruCache<CornflakesMlx5Slab>>, $opt);
            }
            zero_copy_cache::data_structures::CacheType::Mfu => {
                $method!(CornflakesKVServer<Mlx5Connection<zero_copy_cache::data_structures::MfuCache<CornflakesMlx5Slab>>>, Mlx5Connection<zero_copy_cache::data_structures::MfuCache<CornflakesMlx5Slab>>, $opt);
            }
            zero_copy_cache::data_structures::CacheType::NoAlg => {
                $method!(CornflakesKVServer<Mlx5Connection<zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>>>, Mlx5Connection<zero_copy_cache::data_structures::NoAlgCache<CornflakesMlx5Slab>>, $opt);

            }
        }
    }
);
