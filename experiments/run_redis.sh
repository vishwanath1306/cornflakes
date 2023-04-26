#!/bin/bash
python3 /mydata/deeptir/cornflakes/experiments/twitter-bench.py -e loop -f /mydata/deeptir/final_sosp_results/redis_twitter -c /mydata/deeptir/config/cluster_config.yaml -ec /mydata/deeptir/cornflakes/experiments/yamls/cmdlines/redis-twitter.yaml --trace /mydata/deeptir/twitter/cluster4.0_8192.log -lc /mydata/deeptir/cornflakes/experiments/yamls/loopingparams/twitter_traces/cf-kv-twitter-redis.yaml

python3 experiments/cf-kv-bench.py -e loop -f /mydata/deeptir/final_sosp_results/redis_ycsb -c /mydata/deeptir/config/cluster_config.yaml -ec experiments/yamls/cmdlines/redis.yaml -lt /proj/demeter-PG0/deeptir/ycsbc-traces/workloadc-1mil/workloadc-1mil-1-batched.load -qt /proj/demeter-PG0/deeptir/ycsbc-traces/workloadc-1mil/workloadc-1mil-1-batched.access -lc /mydata/deeptir/cornflakes/experiments/yamls/loopingparams/redis-ycsb-4k.yaml