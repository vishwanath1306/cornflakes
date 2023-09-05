from main import runner, utils
import heapq
from result import Ok
import yaml
from pathlib import Path
import os
import parse
import subprocess as sh
import copy
import time
import pandas as pd
import numpy as np
import collections
STRIP_THRESHOLD = 0.03
SERIALIZATION_LIBRARIES = ["cornflakes-dynamic", "cornflakes1c-dynamic",
                           "capnproto", "flatbuffers", "protobuf", "redis"]

class ZCCYCSBIteration(runner.Iteration):
    def __init__(self,
                client_rates,
                avg_size,
                size_distr,
                num_keys,
                num_values,
                num_threads,
                load_trace,
                access_trace,
                extra_zcc_params: runner.ExtraZccParameters,
                log_keys = False,
                log_pinning_map = False,
                trial=None):
        self.avg_size = avg_size
        self.client_rates = client_rates
        self.size_distr = size_distr
        self.num_keys = num_keys
        self.num_threads = num_threads
        self.num_values = num_values
        self.trial = trial
        self.load_trace = load_trace
        self.access_trace = access_trace
        self.extra_zcc_params = extra_zcc_params
        self.system = extra_zcc_params.system
        self.log_keys = log_keys
        self.log_pinning_map = log_pinning_map
    
    def __str__(self):
        return f"system: {self.system}, "\
                f"size_distr: {self.size_distr}, "\
                f"avg_size: {self.avg_size}, "\
                f"num_keys: {self.num_keys}, "\
                f"num_values: {self.num_values}, "\
                f"num_threads: {self.num_threads}, "\
                f"trial: {self.trial}, "\
                f"load_trace: {self.load_trace}, "\
                f"access_trace: {self.access_trace}, "\
                f"zcc_params: {str(self.extra_zcc_params)}"



    def hash(self):
        # hashes every argument EXCEPT for client rates.
        args = [self.size_distr, self.num_keys, self.system,
                self.num_threads, self.num_values,  self.trial, self.load_trace,
                self.access_trace, str(self.extra_zcc_params)]

    def get_iteration_params(self):
        """
        Returns an array of parameters for this experiment.
        """
        params= ["system", "size_distr", "avg_size", "num_keys",
                "num_values", "num_threads",
                "num_clients", "load_trace", "access_trace",
                "offered_load_pps", "offered_load_gbps"]
        params.extend(self.extra_zcc_params.get_iteration_params())
        return params

    def get_iteration_params_values(self):
        offered_load_pps = 0
        for info in self.client_rates:
            rate = info[0]
            num = info[1]
            offered_load_pps += rate * num * self.num_threads
        # convert to gbps
        offered_load_gbps = utils.get_tput_gbps(offered_load_pps,
                self.get_iteration_avg_message_size())
        ret = {
                "system": self.system,
                "size_distr": self.size_distr,
                "avg_size": self.avg_size,
                "num_keys": self.num_keys,
                "num_values": self.num_values,
                "num_threads": self.num_threads,
                "num_clients": self.get_num_clients(),
                "load_trace": self.load_trace,
                "access_trace": self.access_trace,
                "offered_load_pps": offered_load_pps,
                "offered_load_gbps": offered_load_gbps,
            }
        ret.update(self.extra_zcc_params.get_iteration_params_values())
        return ret

    def get_iteration_avg_message_size(self):
        return self.avg_size * self.num_values

    def get_num_threads(self):
        return self.num_threads

    def get_size_distr(self):
        return self.size_distr

    def get_num_values(self):
        return self.num_values

    def get_num_keys(self):
        return self.num_keys

    def get_trial(self):
        return self.trial

    def set_trial(self, trial):
        self.trial = trial

    def get_num_values_string(self):
        return "values_{}".format(self.num_values)

    def get_num_keys_string(self):
        return "keys_{}".format(self.num_keys)

    def get_trial_string(self):
        if self.trial == None:
            utils.error("TRIAL IS NOT SET FOR ITERATION.")
            exit(1)
        return "trial_{}".format(self.trial)

    def get_num_threads_string(self):
        return "{}_threads".format(self.num_threads)

    def get_client_rate_string(self):
        # 2@300000,1@100000 implies 2 clients at 300000 pkts / sec and 1 at
        # 100000 pkts / sec
        ret = ""
        for info in self.client_rates:
            rate = info[0]
            num = info[1]
            if ret != "":
                ret += ","
            ret += "{}@{}".format(num, rate)
        return ret

    def get_num_clients(self):
        total_hosts = 0
        for i in self.client_rates:
            total_hosts += i[1]
        return total_hosts

    def get_iteration_clients(self, possible_hosts):
        total_hosts = 0
        for i in self.client_rates:
            total_hosts += i[1]
        return possible_hosts[0:total_hosts]


    def find_rate(self, client_options, host):
        rates = []
        for info in self.client_rates:
            rate = info[0]
            num = info[1]
            for idx in range(num):
                rates.append(rate)
        try:
            rate_idx = client_options.index(host)
            return rates[rate_idx]
        except:
            utils.error("Host {} not found in client options {}.".format(
                        host,
                        client_options))
            exit(1)

    def get_size_distr_string(self):
        return "size_{}".format(self.size_distr)

    def get_parent_folder(self, high_level_folder):
        path = Path(high_level_folder)
        return path / self.system /\
                self.extra_zcc_params.get_subfolder() /\
                self.get_size_distr_string() /\
            self.get_num_keys_string() /\
            self.get_num_values_string() / \
            self.get_client_rate_string() /\
            self.get_num_threads_string()

    def get_folder_name(self, high_level_folder):
        return self.get_parent_folder(high_level_folder) / self.get_trial_string()

    def get_program_args(self,
                         host,
                         config_yaml,
                         program,
                         programs_metadata):
        ret = {}
        ret["size_distr"] = "{}".format(self.size_distr)
        ret["num_keys"] = "{}".format(self.num_keys)
        ret["num_values"] = "{}".format(self.num_values)
        ret["library"] = "cornflakes-dynamic"
        ret["client_library"] = "cornflakes1c-dynamic"
        folder_name = self.get_folder_name(config_yaml["hosts"][host]["tmp_folder"])
        self.extra_zcc_params.fill_in_args(ret, program)
        host_type_map = config_yaml["host_types"]
        server_host = host_type_map["server"][0]
        if program == "start_server":
            ret["trace"] = self.load_trace
            ret["server_ip"] = config_yaml["hosts"][host]["ip"]
            ret["mode"] = "server"
            ret["log_keys"] = ""
            ret["log_pinning_map"] = ""
            if self.log_keys:
                # must match what is in experiment yaml
                ret["log_keys"] = f" --record_key_mappings {folder_name}/{host}_keymappings.log"
            if self.log_pinning_map:
                ret["log_pinning_map"] = f" --record_pinning_map {folder_name}/{host}_pinning.log"
        elif program == "start_client":
            ret["queries"] = self.access_trace
            ret["mode"] = "client"

            # calculate client rate
            host_options = self.get_iteration_clients(
                    host_type_map["client"])
            rate = self.find_rate(host_options, host)
            ret["rate"] = rate
            ret["num_threads"] = self.num_threads
            ret["num_clients"] = len(host_options)
            ret["num_machines"] = self.get_num_clients()
            ret["machine_id"] = self.find_client_id(host_options, host)

            # calculate server host
            ret["server_ip"] =  config_yaml["hosts"][server_host]["ip"]
            ret["client_ip"] = config_yaml["hosts"][host]["ip"]
        else:
            utils.error("Unknown program name: {}".format(program))
            exit(1)
        return ret


# for each system -- we can vary the following parameters
ZCCKVExpInfo = collections.namedtuple("ZCCKVExpInfo", 
        ["register_at_start",
        "pinning_limit",
        "segment_size", 
        "pinning_frequency"])
        
class ZCCKVBench(runner.Experiment):
    def __init__(self, exp_yaml, config_yaml):
        self.exp = "ZCCKVBench"
        self.exp_yaml = yaml.load(Path(exp_yaml).read_text(),
                Loader=yaml.FullLoader)
        self.config_yaml = yaml.load(Path(config_yaml).read_text(),
                Loader=yaml.FullLoader)

    def experiment_name(self):
        return self.exp

    def skip_iteration(self, total_args, iteration):
        return False

    def append_to_skip_info(self, total_args, iteration, higher_level_folder):
        return

    def parse_exp_info_string(self, exp_string):
        """
        Returns parsed ZCCKVExpInfo from exp_string.
        Should be formatted as:
        register_at_start = {1|0}, pinning_limit = {}, segment_size = {}, pinning_frequency = {}
        num_values = {}, num_keys = {}, size = {}
        """
        try:
            parse_result = parse.parse("register_at_start = {:d}, pinning_limit = {:d}, segment_size = {}, pinning_frequency = {}", exp_string)
            register_at_start = True
            if parse_result[0] == 0:
                register_at_start = False
            elif parse_result[0] == 1:
                register_at_start = True
            else:
                utils.warn("Must pass in 1|0 for register_at_start")
                exit(1)
            return ZCCKVExpInfo(register_at_start, parse_result[1],
                    parse_result[2], parse_result[3])
        except:
            utils.error("Error parsing exp_string: {}".format(exp_string))
            exit(1)

    def get_iterations(self, total_args):
        if total_args.exp_type == "individual":
            if total_args.num_clients > int(self.config_yaml["max_clients"]):
                utils.error("Cannot have {} clients, greater than max {}"
                            .format(total_args.num_clients,
                                    self.config_yaml["max_clients"]))
                exit(1)
            client_rates = [(total_args.rate, total_args.num_clients)]
            value_size = int(total_args.size / total_args.num_values)
            size_distr = "UniformOverSizes-{}".format(value_size)
            extra_zcc_params = runner.ExtraZccParameters(total_args.system,
                    zcc_pinning_limit=total_args.zcc_pinning_budget,
                    zcc_segment_size=total_args.zcc_segment_size,
                    register_at_start=total_args.zcc_register_at_start,
                    zcc_pinning_frequency=total_args.zcc_pinning_frequency)
            it = ZCCYCSBIteration(client_rates,
                             value_size,
                             size_distr,
                             total_args.num_keys,
                             total_args.num_values,
                             total_args.num_threads,
                             total_args.load_trace,
                             total_args.access_trace,
                             extra_zcc_params,
                             log_keys=total_args.log_keys,
                             log_pinning_map=total_args.log_pinning_map
                             )
            num_trials_finished = utils.parse_number_trials_done(
                it.get_parent_folder(total_args.folder))
            if total_args.analysis_only or total_args.graph_only:
                ret = []
                for i in range(0, num_trials_finished):
                    it_clone = copy.deepcopy(it)
                    it_clone.set_trial(i)
                    ret.append(it_clone)
                return ret
            it.set_trial(num_trials_finished)
            return [it]
        else:
            ret = []
            loop_yaml = self.get_loop_yaml()
            # loop over various options
            num_trials = utils.yaml_get(loop_yaml, "num_trials")
            num_threads = utils.yaml_get(loop_yaml, "num_threads")
            num_clients = utils.yaml_get(loop_yaml, "num_clients")
            rate_percentages = utils.yaml_get(loop_yaml, "rate_percentages")
            num_values = utils.yaml_get(loop_yaml, "num_values")
            num_keys = utils.yaml_get(loop_yaml, "num_keys")
            size_str = utils.yaml_get(loop_yaml, "size_str")
            value_size = utils.parse_cornflakes_size_distr_avg(size_str)
            systems = utils.yaml_get(loop_yaml, "systems")
            max_rates_dict = self.parse_max_rates(utils.yaml_get(loop_yaml, "max_rates"))


            for trial in range(num_trials):
                for system in systems:
                    for rate_percentage in rate_percentages:
                        for zcckvexp in max_rates_dict:
                            max_rate = max_rates_dict[zcckvexp]
                            rate = int(float(max_rate) *
                                        rate_percentage)
                            client_rates = [(rate, num_clients)]
                            extra_zcc_params = runner.ExtraZccParameters(system,
                                    zcc_pinning_limit=zcckvexp.pinning_limit,
                                    zcc_segment_size=zcckvexp.segment_size,
                                    register_at_start=zcckvexp.register_at_start,
                                    zcc_pinning_frequency=zcckvexp.pinning_frequency)
                            it = ZCCYCSBIteration(client_rates,
                                value_size,
                                size_str,
                                num_keys,
                                num_values,
                                num_threads,
                                total_args.load_trace,
                                total_args.access_trace,
                                extra_zcc_params,
                                log_keys=total_args.log_keys,
                                log_pinning_map=total_args.log_pinning_map,
                                trial=trial)
                            ret.append(it)
            return ret

    def add_specific_args(self, parser, namespace):
        parser.add_argument("-l", "--logfile",
                            help="logfile name",
                            default="latencies.log")
        parser.add_argument("-lt", "--load_trace",
                            dest="load_trace",
                            required=True)
        parser.add_argument("-qt", "--access_trace",
                            dest="access_trace",
                            required=True)
        parser.add_argument("--log_keys",
                            dest="log_keys",
                            action="store_true",
                            help = "Whether to log key mappings")
        parser.add_argument("--log_pinning_map",
                            dest="log_pinning_map",
                            action="store_true",
                            help = "Whether to log the pinning map at each epoch.")
        if namespace.exp_type == "individual":

            parser.add_argument("-nt", "--num_threads",
                                dest="num_threads",
                                type=int,
                                default=1,
                                help="Number of threads to run with")
            parser.add_argument("-r", "--rate",
                                dest="rate",
                                type=int,
                                default=60000,
                                help="Rate of client(s) in (pkts/sec).")
            parser.add_argument("-s", "--size",
                                dest="size",
                                type=int,
                                help="Total message size.",
                                required=True)
            parser.add_argument("-nk", "--num_keys",
                                dest="num_keys",
                                type=int,
                                default=1,
                                help="Number of keys")
            parser.add_argument("-nv", "--num_values",
                                dest="num_values",
                                type=int,
                                default=1,
                                help="Number of values to batch together")
            parser.add_argument("-nc", "--num_clients",
                                dest="num_clients",
                                type=int,
                                default=1)
            parser = runner.extend_with_zcc_parameters(parser) 
        args = parser.parse_args(namespace=namespace)
        return args

    def get_exp_config(self):
        return self.exp_yaml

    def get_machine_config(self):
        return self.config_yaml

    def exp_post_process_analysis(self, total_args, logfile, new_logfile):
        pass

    def graph_results(self, args, folder, logfile, post_process_logfile):
        pass

def main():
    parser, namespace = runner.get_basic_args()
    kv_bench = ZCCKVBench(
        namespace.exp_config,
        namespace.config)
    kv_bench.execute(parser, namespace)


if __name__ == '__main__':
    main()
