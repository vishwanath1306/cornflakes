use color_eyre::eyre::{bail, Result, WrapErr};
pub mod hardcoded_cf;

/*pub mod kv_messages {
    include!(concat!(env!("OUT_DIR"), "/kv_cf_dynamic.rs"));
}*/

use super::{ycsb_parser::YCSBRequest, KVSerializer, SerializedRequestGenerator};
use cornflakes_codegen::utils::rc_dynamic_hdr::HeaderRepr;
use cornflakes_libos::{
    dpdk_bindings::rte_memcpy_wrapper as rte_memcpy, CfBuf, Datapath, RcCornPtr, RcCornflake,
    ReceivedPkt, ScatterGather,
};
use hashbrown::HashMap;

// empty object
pub struct CornflakesDynamicSerializer;

impl<D> KVSerializer<D> for CornflakesDynamicSerializer
where
    D: Datapath,
{
    type HeaderCtx = Vec<u8>;

    fn new(_serialize_to_native_buffers: bool) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(CornflakesDynamicSerializer {})
    }

    fn handle_get<'a>(
        &self,
        pkt: ReceivedPkt<D>,
        _map: &HashMap<String, CfBuf<D>>,
        num_values: usize,
    ) -> Result<(Self::HeaderCtx, RcCornflake<'a, D>)> {
        match num_values {
            0 => {
                bail!("Number of get values cannot be 0");
            }
            1 => {
                let mut get_request = hardcoded_cf::GetReq::<D>::new();
                get_request.deserialize(&pkt)?;
                let value = match map.get(

                let mut response = hardcoded_cf::GetResp::<D>::new();
                response.set_id(get_request.get_id());

                // serialize the request
                let (header_vec, cf) = response.serialize(rte_memcpy)?;
                return Ok((header_vec, cf));
            }
            _x => {
                bail!("Not implemented");
            }
        }

        //Ok((Vec::default(), RcCornflake::default()))
    }

    fn handle_put<'a>(
        &self,
    _pkt: ReceivedPkt<D>,
        _map: &mut HashMap<String, CfBuf<D>>,
    _num_values: usize,
    ) -> Result<(Self::HeaderCtx, RcCornflake<'a, D>)> {
        // for the "copy-out" deserialization/insert:
        //      - just need to allocate the buffer from the networking stack
        //      - and copy the get_value() value into this buffer
        // for ref counting deserialization / insert: where we insert reference to the entire
        // packet
        //      - serialization should be ref counting aware
        //      - getter: needs to generate an CfBuf<D> to insert into kv store -- but how????
        //      - get_value() -> needs to return a buffer that *references* a datapath packet
        //      - the deserialized value should keep a reference to the buffer???
        //      - but also to the datapath packet???
        //      - for the split receive: the received thing could *also* be a scatter-gather array
        //      e.g. GetResponse<D> where D: Datapath {
        //          buf: CfBuf<D>,
        //          has_header_ptr: bool,
        //          bitmap: Vec<u8>,
        //          id: u32,
        //          bytes_field:
        //      }
        //      but then how is an individual element in that struct also a CfBuf???? not clear
        //
        Ok((Vec::default(), RcCornflake::default()))
    }

    fn process_header<'a>(
        &self,
        ctx: &'a Self::HeaderCtx,
        cornflake: &mut RcCornflake<'a, D>,
    ) -> Result<()> {
        cornflake.replace(0, RcCornPtr::RawRef(ctx.as_slice()));
        Ok(())
    }
}
