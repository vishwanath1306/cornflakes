python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load \
-qt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system vanilla_cornflakes \
--zcc_pinning_budget 1024 \
--zcc_segment_size 64 \
--pprint

# Server 

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --zcc_pinning_limit 64000 --zcc_segment_size 64 --zcc_alg noalg --zcc_sleep_duration 1000 

# Client 

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 6250 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list 

# == Cornflakes MFU + 1024 PB ==============


python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load \
-qt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 1024 \
--zcc_segment_size 64 \
--pprint

# server

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --dont_register_at_start --zcc_pinning_limit 1024 --zcc_segment_size 64 --zcc_alg mfu --zcc_sleep_duration 1000


# Client

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 6250 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list

# ======= Cornflakes MFU + 512 PB =========

python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load \
-qt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 512 \
--zcc_segment_size 64 \
--pprint

# Server

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --dont_register_at_start --zcc_pinning_limit 512 --zcc_segment_size 64 --zcc_alg mfu --zcc_sleep_duration 1000

# Client
 sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 12500 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list

# ======= Cornflakes MFU + 256 PB =========

python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load \
-qt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 256 \
--zcc_segment_size 64 \
--pprint

# Server 

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --dont_register_at_start --zcc_pinning_limit 256 --zcc_segment_size 64 --zcc_alg mfu --zcc_sleep_duration 1000

# Client 

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 6250 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list


# == Vanilla Cornflakes + 500k 20p hs workload ==============


python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_500k_hs/vish_500k_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_500k_hs/vish_500k_hs-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system vanilla_cornflakes \
--zcc_pinning_budget 64000 \
--zcc_segment_size 64 \
--pprint

# Server

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /proj/demeter-PG0/vish/vish_500k_hs/vish_500k_hs-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --zcc_pinning_limit 64000 --zcc_segment_size 64 --zcc_alg noalg --zcc_sleep_duration 1000

# Client

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /proj/demeter-PG0/vish/vish_500k_hs/vish_500k_hs-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 6250 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0

# == Vanilla Cornflakes + 1m 20p hs workload ==============


python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system vanilla_cornflakes \
--zcc_pinning_budget 64000 \
--zcc_segment_size 64 \
--pprint

# Server

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --zcc_pinning_limit 64000 --zcc_segment_size 64 --zcc_alg noalg --zcc_sleep_duration 1000

# Client 

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 62500 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list

# ======= Cornflakes MFU + 512 PB =========

python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load \
-qt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 512 \
--zcc_segment_size 64 \
--pprint

# Server 
sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --dont_register_at_start --zcc_pinning_limit 512 --zcc_segment_size 64 --zcc_alg mfu --zcc_sleep_duration 1000

# Client
sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 6250 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list

# ======= Cornflakes MFU + 1024 PB =========

python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load \
-qt /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 1024 \
--zcc_segment_size 64 \
--pprint

# Server
sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --dont_register_at_start --zcc_pinning_limit 1024 --zcc_segment_size 64 --zcc_alg mfu --zcc_sleep_duration 1000 

# Client

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /mydata/vishwa/data/ycsb/workloadc-1mil/workloadc-1mil-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 6250 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list


# ======= ZCC Cornflakes MFU + 1m 20p hs workload + 256 PB ==============


python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 256 \
--zcc_segment_size 64 \
--pprint

# Server

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --dont_register_at_start --zcc_pinning_limit 256 --zcc_segment_size 64 --zcc_alg mfu --zcc_sleep_duration 1000

#Client
sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 50000 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list

# ======= ZCC Cornflakes MFU + 1m 20p hs workload + 128 PB ==============


python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 128 \
--zcc_segment_size 64 \
--pprint

# Server

sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --dont_register_at_start --zcc_pinning_limit 128 --zcc_segment_size 64 --zcc_alg mfu --zcc_sleep_duration 1000 

# Client
sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 12500 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list

# ======= Cornflakes Copy ==============


python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-nc 1 --num_threads 16 \
--rate 6250 \
--size 2048 \
--num_keys 1 --num_values 1 \
--system cornflakes_copy \
--zcc_pinning_budget 128 \
--zcc_segment_size 64 \
--pprint

# Server 
sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu nice -n -19 taskset -c 2 /mydata/cornflakes/target/release/ycsb_mlx5 --config_file /mydata/cornflakes/vish_config.yaml --server_ip 192.168.1.1 --mode server --trace /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load --debug_level info --value_size UniformOverSizes-2048 --num_values 1 --num_keys 1 --serialization cornflakes1c-dynamic --push_buf_type hybridarenaobject --inline_mode nothing --copy_threshold 512 --use_linked_list --num_pages 64 --zcc_pinning_limit 64000 --zcc_segment_size 64 --zcc_alg noalg --zcc_sleep_duration 1000

# Client
sudo env LD_LIBRARY_PATH=/mydata/cornflakes/dpdk-datapath/3rdparty/dpdk/build/lib/x86_64-linux-gnu /mydata/cornflakes/target/release/ycsb_dpdk --config_file /mydata/cornflakes/vish_config.yaml --mode client --queries /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access --debug_level info --push_buf_type singlebuf --value_size UniformOverSizes-2048 --rate 12500 --serialization cornflakes1c-dynamic --server_ip 192.168.1.1 --our_ip 192.168.1.2 --time 25 --num_values 1 --num_keys 1 --num_threads 16 --num_clients 1 --client_id 0 --use_linked_list


# ======================= THURSDAY CALL COMMANDS ============

python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e individual \
-f /mydata/hotspot-results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-nc 1 \
--num_threads 2 \
--rate 1000 \
--size 4096 \
--num_keys 1 \
--num_values 1 \
--system zcc_cornflakes_mfu \
--zcc_pinning_budget 6400 \
--zcc_segment_size 64

# ======================= ZCC KV Looping Params ============

python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/hotspot-results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/synthetic.yaml \
--pprint


# ======================= ZCC KV Looping Params + 64 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_ss32_results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/synthetic.yaml &

# ======================= ZCC KV Looping Params + 32 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_ss32_results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/0cc-synthetic-32.yaml &

# ======================= ZCC KV Looping Params + 16 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_ss16_results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_1m_hs/vish_1m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/0cc-synthetic-16.yaml &

# ======================= ZCC KV Looping Params + 64 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_8m_ss64 \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/synthetic.yaml &

# ======================= 8m HS workload + ZCC KV Looping Params + 16 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_8m_ss16_results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/0cc-synthetic-16.yaml &


# ======================= 8m HS workload + ZCC KV Looping Params + 16 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_8m_ss128_results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/0cc-synthetic-128.yaml &


# ======================= 8m HS workload + ZCC KV Looping Params + 128 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_8m_ss128_results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/0cc-synthetic-128.yaml &

# ======================= 8m HS workload + ZCC KV Looping Params + 256 Segment Size ============

nohup python3 /mydata/cornflakes/experiments/zcc-cf-kv-bench.py \
-e loop \
-f /mydata/looping_params_8m_ss256_results \
-c /mydata/cornflakes/vish_config.yaml \
-ec /mydata/cornflakes/experiments/yamls/cmdlines/0cc/0cc-ycsb.yaml \
-lt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.load \
-qt /proj/demeter-PG0/vish/vish_8m_hs/vish_8m_hs-1-batched.access \
-lc /mydata/cornflakes/experiments/yamls/loopingparams/0cc/0cc-synthetic-256.yaml &