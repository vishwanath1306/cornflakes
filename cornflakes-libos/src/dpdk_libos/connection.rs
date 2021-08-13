use super::{
    super::{
        dpdk_bindings::*,
        dpdk_call, mbuf_slice, mem,
        timing::{record, timefunc, HistogramWrapper},
        utils, CornType, Datapath, MsgID, PtrAttributes, ReceivedPkt, RefCnt, ScatterGather,
    },
    dpdk_utils, wrapper,
};
use color_eyre::eyre::{bail, Result, WrapErr};
use cornflakes_utils::AppMode;
use eui48::MacAddress;
use hashbrown::HashMap;
use std::{
    net::Ipv4Addr,
    ptr, slice,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::warn;

const MAX_ENTRIES: usize = 60;
const PROCESSING_TIMER: &str = "E2E_PROCESSING_TIME";
const RX_BURST_TIMER: &str = "RX_BURST_TIMER";
const TX_BURST_TIMER: &str = "TX_BURST_TIMER";
const PKT_CONSTRUCT_TIMER: &str = "PKT_CONSTRUCT_TIMER";
const POP_PROCESSING_TIMER: &str = "POP_PROCESSING_TIMER";
const PUSH_PROCESSING_TIMER: &str = "PUSH_PROCESSING_TIMER";

#[derive(PartialEq, Eq)]
pub struct DPDKBuffer {
    /// Pointer to allocated mbuf.
    pub mbuf: *mut rte_mbuf,
    /// Id of originating mempool (application and datapath context).
    pub mempool_id: usize,
    /// Actual application data offset (header could be in front)
    pub offset: usize,
}

impl DPDKBuffer {
    fn new(mbuf: *mut rte_mbuf, mempool_id: usize, data_offset: usize) -> Self {
        DPDKBuffer {
            mbuf: mbuf,
            mempool_id: mempool_id,
            offset: data_offset,
        }
    }
}

impl Default for DPDKBuffer {
    fn default() -> Self {
        DPDKBuffer {
            // TODO: might be safest to NOT have this function
            mbuf: ptr::null_mut(),
            mempool_id: 0,
            offset: 0,
        }
    }
}

impl Drop for DPDKBuffer {
    fn drop(&mut self) {
        // decrement the reference count of the mbuf, or if at 1 or 0, free it
        wrapper::free_mbuf(self.mbuf);
    }
}

impl Clone for DPDKBuffer {
    fn clone(&self) -> DPDKBuffer {
        dpdk_call!(rte_pktmbuf_refcnt_update(self.mbuf, 1));
        DPDKBuffer {
            mbuf: self.mbuf,
            mempool_id: self.mempool_id,
            offset: self.offset,
        }
    }
}

impl std::fmt::Debug for DPDKBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Mbuf addr: {:?}, off: {}", self.mbuf, self.offset)
    }
}

impl RefCnt for DPDKBuffer {
    fn change_rc(&mut self, amt: isize) {
        dpdk_call!(rte_pktmbuf_refcnt_update(self.mbuf, amt as i16));
    }

    fn count_rc(&self) -> usize {
        dpdk_call!(rte_pktmbuf_refcnt_read(self.mbuf)) as usize
    }
}

impl AsRef<[u8]> for DPDKBuffer {
    fn as_ref(&self) -> &[u8] {
        let data_len = unsafe { (*self.mbuf).data_len } as usize;
        let slice = mbuf_slice!(self.mbuf, self.offset, data_len - self.offset);
        tracing::debug!(
            "Mbuf address: {:?}, slice address: {:?}, data off: {:?}, buf_addr: {:?}",
            self.mbuf,
            slice.as_ptr(),
            unsafe { (*self.mbuf).data_off },
            unsafe { (*self.mbuf).buf_addr }
        );
        slice
    }
}

impl AsMut<[u8]> for DPDKBuffer {
    fn as_mut(&mut self) -> &mut [u8] {
        let data_len = unsafe { (*self.mbuf).data_len } as usize;
        let slice = mbuf_slice!(self.mbuf, self.offset, data_len - self.offset);
        tracing::debug!(
            "Mbuf address: {:?}, slice address: {:?}, data off: {:?}, buf_addr: {:?}",
            self.mbuf,
            slice.as_ptr(),
            unsafe { (*self.mbuf).data_off },
            unsafe { (*self.mbuf).buf_addr }
        );
        slice
    }
}

impl PtrAttributes for DPDKBuffer {
    fn buf_size(&self) -> usize {
        unsafe { (*self.mbuf).data_len as usize }
    }

    fn buf_type(&self) -> CornType {
        CornType::Registered
    }
}

pub struct DPDKConnection {
    /// Whether to use scatter-gather on send.
    use_scatter_gather: bool,
    /// Server or client mode.
    mode: AppMode,
    /// dpdk_port
    dpdk_port: u16,
    /// Maps ip addresses to corresponding mac addresses.
    ip_to_mac: HashMap<Ipv4Addr, MacAddress>,
    /// Current window of outgoing packets mapped to start time.
    outgoing_window: HashMap<MsgID, Instant>,
    /// mempools for allocating mbufs.
    /// The default mempool (also used for RX) sits at idx 0.
    mempools: Vec<(usize, *mut rte_mempool)>,
    /// Vector of mempool information.
    memzones: Vec<(usize, (usize, usize))>,
    /// Empty mempool for allocating external buffers.
    extbuf_mempool: *mut rte_mempool,
    /// Header information
    addr_info: utils::AddressInfo,
    /// Registered memory regions for externally allocated memory
    external_memory_regions: Vec<mem::MmapMetadata>,
    /// Debugging timers.
    timers: HashMap<String, Arc<Mutex<HistogramWrapper>>>,
    /// Mbufs used tx_burst.
    send_mbufs: [[*mut rte_mbuf; wrapper::RECEIVE_BURST_SIZE as usize]; wrapper::MAX_SCATTERS],
    /// Mbufs used for rx_burst.
    recv_mbufs: [*mut rte_mbuf; wrapper::RECEIVE_BURST_SIZE as usize],
}

impl DPDKConnection {
    /// Returns a new DPDK connection, or error if there was any problem in initializing and
    /// configuring DPDK.
    /// Also initializes a stub rte_mbuf_ext_shared_info for any external buffers which will be
    /// used to send packets.
    ///
    /// Arguments:
    /// * config_file: String slice representing a path to a config file, in yaml format, that
    /// contains:
    /// (1) A list of mac address and IP addresses in the network.
    /// (2) DPDK rte_eal_init information.
    /// (3) UDP port information for UDP packet headers.
    pub fn new(
        config_file: &str,
        mode: AppMode,
        use_scatter_gather: bool,
    ) -> Result<DPDKConnection> {
        let (ip_to_mac, mac_to_ip, udp_port) = dpdk_utils::parse_yaml_map(config_file).wrap_err(
            "Failed to get ip to mac address mapping, or udp port information from yaml config.",
        )?;
        let outgoing_window: HashMap<MsgID, Instant> = HashMap::new();
        let (mempool, ext_mempool, nb_ports) =
            wrapper::dpdk_init(config_file).wrap_err("Failed to dpdk initialization.")?;

        // TODO: figure out a way to have a "proper" port_id arg
        let my_ether_addr =
            wrapper::get_my_macaddr(nb_ports - 1).wrap_err("Failed to get my own mac address")?;
        let my_mac_addr = MacAddress::from_bytes(&my_ether_addr.addr_bytes)?;
        let my_ip_addr = match mac_to_ip.get(&my_mac_addr) {
            Some(ip) => ip,
            None => {
                bail!(
                    "Not able to find ip addr for my mac addr {:?} in map",
                    my_mac_addr.to_hex_string()
                );
            }
        };

        let addr_info = utils::AddressInfo::new(udp_port, *my_ip_addr, my_mac_addr);

        // initialize any debugging histograms
        // process::exit,
        let mut timers: HashMap<String, Arc<Mutex<HistogramWrapper>>> = HashMap::default();
        if cfg!(feature = "timers") {
            if mode == AppMode::Server {
                timers.insert(
                    PROCESSING_TIMER.to_string(),
                    Arc::new(Mutex::new(HistogramWrapper::new(PROCESSING_TIMER)?)),
                );
                timers.insert(
                    POP_PROCESSING_TIMER.to_string(),
                    Arc::new(Mutex::new(HistogramWrapper::new(POP_PROCESSING_TIMER)?)),
                );
            }
            timers.insert(
                RX_BURST_TIMER.to_string(),
                Arc::new(Mutex::new(HistogramWrapper::new(RX_BURST_TIMER)?)),
            );
            timers.insert(
                PKT_CONSTRUCT_TIMER.to_string(),
                Arc::new(Mutex::new(HistogramWrapper::new(PKT_CONSTRUCT_TIMER)?)),
            );
            timers.insert(
                TX_BURST_TIMER.to_string(),
                Arc::new(Mutex::new(HistogramWrapper::new(TX_BURST_TIMER)?)),
            );
            timers.insert(
                PUSH_PROCESSING_TIMER.to_string(),
                Arc::new(Mutex::new(HistogramWrapper::new(PUSH_PROCESSING_TIMER)?)),
            );
        }

        let mempools = vec![(0, mempool)];
        let memzones = vec![(0, wrapper::get_mempool_memzone_area(mempool)?)];

        Ok(DPDKConnection {
            use_scatter_gather: use_scatter_gather,
            mode: mode,
            dpdk_port: nb_ports - 1,
            ip_to_mac: ip_to_mac,
            //mac_to_ip: mac_to_ip,
            outgoing_window: outgoing_window,
            mempools: mempools,
            memzones: memzones,
            external_memory_regions: Vec::default(),
            extbuf_mempool: ext_mempool,
            addr_info: addr_info,
            //shared_info: HashMap::new(),
            timers: timers,
            send_mbufs: [[ptr::null_mut(); wrapper::RECEIVE_BURST_SIZE as usize];
                wrapper::MAX_SCATTERS],
            recv_mbufs: [ptr::null_mut(); wrapper::RECEIVE_BURST_SIZE as usize],
        })
    }

    fn get_timer(
        &self,
        timer_name: &str,
        cond: bool,
    ) -> Result<Option<Arc<Mutex<HistogramWrapper>>>> {
        if !cond {
            return Ok(None);
        }
        match self.timers.get(timer_name) {
            Some(h) => Ok(Some(h.clone())),
            None => bail!("Failed to find timer {}", timer_name),
        }
    }

    fn start_entry(&mut self, timer_name: &str, id: MsgID, src: Ipv4Addr) -> Result<()> {
        let mut hist = match self.timers.contains_key(timer_name) {
            true => match self.timers.get(timer_name).unwrap().lock() {
                Ok(h) => h,
                Err(e) => bail!("Failed to unlock hist: {}", e),
            },
            false => {
                bail!("Entry not in timer map: {}", timer_name);
            }
        };
        hist.start_entry(src, id)?;
        Ok(())
    }

    fn end_entry(&mut self, timer_name: &str, id: MsgID, dst: Ipv4Addr) -> Result<()> {
        let mut hist = match self.timers.contains_key(timer_name) {
            true => match self.timers.get(timer_name).unwrap().lock() {
                Ok(h) => h,
                Err(e) => bail!("Failed to unlock hist: {}", e),
            },
            false => {
                bail!("Entry not in timer map: {}", timer_name);
            }
        };
        hist.end_entry(dst, id)?;
        Ok(())
    }

    fn get_outgoing_header(&self, dst_addr: &utils::AddressInfo) -> utils::HeaderInfo {
        self.addr_info
            .get_outgoing(dst_addr.ipv4_addr, dst_addr.ether_addr)
    }

    pub fn add_mempool(&mut self, value_size: usize, min_num_values: usize) -> Result<()> {
        // Adds a mempool to the mempool allocator of this size.
        // Mainly for the KV implementation.
        unimplemented!();
    }
}

impl Datapath for DPDKConnection {
    type DatapathPkt = DPDKBuffer;
    /// Sends a single buffer to the given address.
    fn push_buf(&mut self, buf: (MsgID, &[u8]), addr: utils::AddressInfo) -> Result<()> {
        let header = self.get_outgoing_header(&addr);
        self.send_mbufs[0][0] =
            wrapper::get_mbuf_with_memcpy(self.mempools[0].1, &header, buf.1, buf.0)?;

        // if client, add start time for packet
        // if server, end packet processing counter
        match self.mode {
            AppMode::Server => {
                if cfg!(feature = "timers") {
                    self.end_entry(PROCESSING_TIMER, buf.0, addr.ipv4_addr)?;
                }
            }
            AppMode::Client => {
                // only insert new time IF this packet has not already been sent
                if !self.outgoing_window.contains_key(&buf.0) {
                    let _ = self.outgoing_window.insert(buf.0, Instant::now());
                }
            }
        }

        // send out the scatter-gather array
        let mbuf_ptr = &mut self.send_mbufs[0][0] as _;
        timefunc(
            &mut || {
                wrapper::tx_burst(self.dpdk_port, 0, mbuf_ptr, 1)
                    .wrap_err(format!("Failed to send SGAs."))
            },
            cfg!(feature = "timers"),
            self.get_timer(TX_BURST_TIMER, cfg!(feature = "timers"))?,
        )?;

        Ok(())
    }

    /// Sends out a cornflake to the given Ipv4Addr.
    /// Returns an error if the address is not present in the ip_to_mac table,
    /// or if there is a problem constructing a linked list of mbufs to copy/attach the cornflake
    /// data to.
    ///
    /// Arguments:
    /// * sga - reference to a cornflake which contains the scatter-gather array to send
    /// out.
    /// * addr - Ipv4Addr to send the given scatter-gather array to.
    fn push_sgas(&mut self, sgas: &Vec<(impl ScatterGather, utils::AddressInfo)>) -> Result<()> {
        let push_processing_start = Instant::now();
        let push_processing_timer =
            self.get_timer(PUSH_PROCESSING_TIMER, cfg!(feature = "timers"))?;
        record(
            push_processing_timer,
            push_processing_start.elapsed().as_nanos() as u64,
        )?;
        let mut pkts: Vec<wrapper::Pkt> =
            sgas.iter().map(|(_sga, _)| wrapper::Pkt::init()).collect();

        let headers: Vec<utils::HeaderInfo> = sgas
            .iter()
            .map(|(_, addr)| self.get_outgoing_header(addr))
            .collect();
        let pkt_construct_timer = self.get_timer(PKT_CONSTRUCT_TIMER, cfg!(feature = "timers"))?;
        let use_scatter_gather = self.use_scatter_gather;
        timefunc(
            &mut || {
                for (i, (((ref sga, _), ref header), ref mut pkt)) in sgas
                    .iter()
                    .zip(headers.iter())
                    .zip(pkts.iter_mut())
                    .enumerate()
                {
                    if use_scatter_gather {
                        pkt.construct_from_sga(
                            &mut self.send_mbufs,
                            i,
                            sga,
                            self.mempools[0].1,
                            self.extbuf_mempool,
                            header,
                            &self.memzones,
                            &self.external_memory_regions,
                        )
                        .wrap_err(format!(
                            "Unable to construct pkt from sga with scatter-gather, sga idx: {}",
                            sga.get_id()
                        ))?;
                    } else {
                        pkt.construct_from_sga_without_scatter_gather(
                            &mut self.send_mbufs,
                            i,
                            sga,
                            self.mempools[0].1,
                            header,
                        )
                        .wrap_err(format!(
                            "Unable to construct pkt from sga without scatter-gather, sga idx: {}",
                            sga.get_id()
                        ))?;
                    }
                }
                Ok(())
            },
            cfg!(feature = "timers"),
            pkt_construct_timer,
        )?;
        tracing::debug!("Constructed packet.");

        // if client, add start time for packet
        // if server, end packet processing counter
        match self.mode {
            AppMode::Server => {
                if cfg!(feature = "timers") {
                    for (sga, addr) in sgas.iter() {
                        self.end_entry(PROCESSING_TIMER, sga.get_id(), addr.ipv4_addr)?;
                    }
                }
            }
            AppMode::Client => {
                for (sga, _) in sgas.iter() {
                    // only insert new time IF this packet has not already been sent
                    if !self.outgoing_window.contains_key(&sga.get_id()) {
                        let _ = self.outgoing_window.insert(sga.get_id(), Instant::now());
                    }
                }
            }
        }

        // send out the scatter-gather array
        let mbuf_ptr = &mut self.send_mbufs[0][0] as _;
        timefunc(
            &mut || {
                wrapper::tx_burst(self.dpdk_port, 0, mbuf_ptr, sgas.len() as u16)
                    .wrap_err(format!("Failed to send SGAs."))
            },
            cfg!(feature = "timers"),
            self.get_timer(TX_BURST_TIMER, cfg!(feature = "timers"))?,
        )?;

        Ok(())
    }

    /// Checks to see if any packet has arrived, if any packet is valid.
    /// Feturns a Vec<(DPDKReceivedPkt, Duration)> for each valid packet.
    /// For client mode, provides duration since sending sga with this id.
    /// FOr server mode, returns 0 duration.
    fn pop(&mut self) -> Result<Vec<(ReceivedPkt<Self>, Duration)>> {
        let received = wrapper::rx_burst(
            self.dpdk_port,
            0,
            self.recv_mbufs.as_mut_ptr(),
            wrapper::RECEIVE_BURST_SIZE,
            &self.addr_info,
        )
        .wrap_err("Error on calling rte_eth_rx_burst.")?;
        let mut ret: Vec<(ReceivedPkt<Self>, Duration)> = Vec::new();

        // Debugging end to end processing time
        if cfg!(feature = "timers") && self.mode == AppMode::Server {
            for (_, (msg_id, addr_info)) in received.iter() {
                self.start_entry(PROCESSING_TIMER, *msg_id, addr_info.ipv4_addr.clone())?;
            }
        }

        if received.len() > 0 {
            tracing::debug!("Received some packs");
            let pop_processing_timer = self.get_timer(
                POP_PROCESSING_TIMER,
                cfg!(feature = "timers") && self.mode == AppMode::Server,
            )?;
            let start = Instant::now();
            for (idx, (msg_id, addr_info)) in received.into_iter() {
                let mbuf = self.recv_mbufs[idx];
                if mbuf.is_null() {
                    bail!("Mbuf for index {} in returned array is null.", idx);
                }

                // for now, this datapath just returns single packets without split receive
                let received_buffer = vec![DPDKBuffer::new(
                    self.recv_mbufs[idx],
                    0,
                    utils::TOTAL_HEADER_SIZE,
                )];

                let received_pkt = ReceivedPkt::new(received_buffer, msg_id, addr_info);

                let duration = match self.mode {
                    AppMode::Client => match self.outgoing_window.remove(&msg_id) {
                        Some(start) => start.elapsed(),
                        None => {
                            warn!("Received packet for an old msg_id: {}", msg_id);
                            continue;
                        }
                    },
                    AppMode::Server => Duration::new(0, 0),
                };
                ret.push((received_pkt, duration));
            }
            record(pop_processing_timer, start.elapsed().as_nanos() as u64)?;
        }
        Ok(ret)
    }

    /// Checks if any outstanding Cornflake has timed out.
    /// Returns a vector with the IDs of any timed-out Cornflakes.
    ///
    /// Arguments:
    /// * time_out - std::time::Duration that represents the timeout period to check for.
    fn timed_out(&self, time_out: Duration) -> Result<Vec<MsgID>> {
        let mut timed_out: Vec<MsgID> = Vec::new();
        for (id, start) in self.outgoing_window.iter() {
            if start.elapsed().as_nanos() > time_out.as_nanos() {
                tracing::debug!(elapsed = ?start.elapsed().as_nanos(), id = *id, "Timing out");
                timed_out.push(*id);
            }
        }
        Ok(timed_out)
    }

    /// Returns the current cycles since boot.
    /// Use rte_get_timer_hz() to know the number of cycles per second.
    fn current_cycles(&self) -> u64 {
        dpdk_call!(rte_get_timer_cycles())
    }

    /// Number of cycles per second.
    /// Can ve used in conjunction with `current_cycles` for time.
    fn timer_hz(&self) -> u64 {
        dpdk_call!(rte_get_timer_hz())
    }

    /// The maximum number of scattered segments that this datapath supports.
    fn max_scatter_entries(&self) -> usize {
        return MAX_ENTRIES;
    }

    /// Maxmimum packet length this datapath supports.
    /// We do not yet support sending payloads larger than an MTU.
    fn max_packet_len(&self) -> usize {
        return wrapper::RX_PACKET_LEN as usize;
    }

    /// Registers this external piece of memory with DPDK,
    /// so regions of this memory can be used while sending external mbufs.
    fn register_external_region(&mut self, metadata: &mut mem::MmapMetadata) -> Result<()> {
        let mut lkey_out: u32 = 0;
        let ibv_mr = wrapper::dpdk_register_extmem(&metadata, &mut lkey_out as *mut u32)?;
        metadata.set_lkey(lkey_out);
        metadata.set_ibv_mr(ibv_mr);
        self.external_memory_regions.push(metadata.clone());
        Ok(())
    }

    fn unregister_external_region(&mut self, metadata: &mem::MmapMetadata) -> Result<()> {
        let mut idx_to_remove = 0;
        let mut found = false;
        for (idx, meta) in self.external_memory_regions.iter().enumerate() {
            if meta.ptr == metadata.ptr && meta.length == metadata.length {
                idx_to_remove = idx;
                found = true;
                break;
            }
        }
        if !found {
            bail!("Could not find external memory region to remove.");
        }
        let metadata = self.external_memory_regions.remove(idx_to_remove);
        wrapper::dpdk_unregister_extmem(&metadata)?;
        Ok(())
    }

    fn get_timers(&self) -> Vec<Arc<Mutex<HistogramWrapper>>> {
        self.timers.iter().map(|(_, hist)| hist.clone()).collect()
    }

    /// Returns a HeaderInfo struct with udp, ethernet and ipv4 header information.
    ///
    /// Arguments:
    /// * dst_addr - Ipv4Addr that is the destination.
    ///
    /// Returns:
    ///  * AddressInfo - struct with destination mac, ip address and udp port
    fn get_outgoing_addr_from_ip(&self, dst_addr: Ipv4Addr) -> Result<utils::AddressInfo> {
        match self.ip_to_mac.get(&dst_addr) {
            Some(mac) => Ok(utils::AddressInfo::new(
                self.addr_info.udp_port,
                dst_addr,
                *mac,
            )),
            None => {
                bail!("Don't know ethernet address for Ip address: {:?}", dst_addr);
            }
        }
    }

    fn get_header_size(&self) -> usize {
        utils::TOTAL_HEADER_SIZE
    }

    fn allocate(&self, size: usize, _align: usize) -> Result<Self::DatapathPkt> {
        // TODO: actually have this allocate from different mempools based on size.
        if size > wrapper::MBUF_BUF_SIZE as usize {
            bail!("Cannot allocate request with size: {:?}", size);
        }
        let mempool = self.mempools[0].1;
        let mbuf = wrapper::alloc_mbuf(mempool)
            .wrap_err(format!("Unable to alloc mbuf from mempool # {}", 0))?;
        tracing::debug!(
            "Allocating DPDK buffer at address {:?} from mempool {:?}",
            mbuf,
            0
        );
        return Ok(DPDKBuffer {
            mbuf: mbuf,
            mempool_id: 0,
            offset: 0,
        });
    }
}

/// When the DPDKConnection goes out of scope,
/// we make sure that the underlying mempools are freed as well.
impl Drop for DPDKConnection {
    fn drop(&mut self) {
        tracing::debug!("DPDK connection is being dropped");
        for metadata in self.external_memory_regions.iter() {
            match wrapper::dpdk_unregister_extmem(metadata) {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(metadata = ?metadata, e = ?e, "Error from calling unregister extmem");
                }
            }
        }
        wrapper::free_mempool(self.extbuf_mempool);
        for (_, mempool) in self.mempools.iter() {
            wrapper::free_mempool(*mempool);
        }
    }
}
