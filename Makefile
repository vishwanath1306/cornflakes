all: build

# TODO: make it so that if mlx5 drivers are not present on this machine, it only
# tries to build the dpdk version of things

build: mlx5-datapath
	cargo b --release

.PHONY: mlx5-datapath mlx5-netperf

mlx5-datapath:
	$(MAKE) -C mlx5-datapath/mlx5-wrapper CONFIG_MLX5=$(CONFIG_MLX5) DEBUG=$(DEBUG)

# mlx5 netperf microbenchmark
mlx5-netperf:
	$(MAKE) -C mlx5-netperf CONFIG_MLX5=$(CONFIG_MLX5) DEBUG=$(DEBUG)

# clean up the system and components
clean:
	rm -rf mlx5-datapath/mlx5-wrapper/rdma-core/build
	$(MAKE) -C mlx5-datapath/mlx5-wrapper clean
	# TODO: also clean up DPDK
	cargo clean

# initialize all of the submodules
submodules:
	# build rdma-core
	git submodule init
	git submodule update --init -f --recursive
	$(MAKE) submodules -C mlx5-datapath/mlx5-wrapper
	# apply the DPDK patch
	git -C cornflakes-libos/3rdparty/dpdk apply ../../dpdk-mlx.patch
	# build DPDK
	cornflakes-libos/build-dpdk.sh cornflakes-libos/3rdparty/dpdk
	
	


