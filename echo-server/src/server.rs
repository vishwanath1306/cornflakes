use super::CerealizeMessage;
use color_eyre::eyre::{Result, WrapErr};
use cornflakes_libos::{
    timing::HistogramWrapper, utils::AddressInfo, Datapath, RcCornflake, ReceivedPkt,
    ScatterGather, ServerSM,
};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct EchoServer<S, D> {
    serializer: S,
    _marker: PhantomData<D>,
}

impl<S, D> EchoServer<S, D>
where
    S: CerealizeMessage<D>,
    D: Datapath,
{
    pub fn new(serializer: S) -> EchoServer<S, D> {
        EchoServer {
            serializer: serializer,
            _marker: PhantomData,
        }
    }
}

impl<S, D> ServerSM for EchoServer<S, D>
where
    S: CerealizeMessage<D>,
    D: Datapath,
{
    type Datapath = D;

    fn init(&mut self, _connection: &mut Self::Datapath) -> Result<()> {
        // for this app, no need to do datapath specific initialization
        // application doesn't end up using any external memory
        Ok(())
    }

    fn cleanup(&mut self, _connection: &mut Self::Datapath) -> Result<()> {
        Ok(())
    }

    fn process_requests(
        &mut self,
        sgas: Vec<(ReceivedPkt<<Self as ServerSM>::Datapath>, Duration)>,
        conn: &mut D,
    ) -> Result<()> {
        let mut out_sgas: Vec<(RcCornflake<D>, AddressInfo)> = Vec::with_capacity(sgas.len());
        let mut contexts: Vec<S::Ctx> = Vec::default();
        for (_i, (in_sga, _)) in sgas.iter().enumerate() {
            let (header_ctx, mut out_sga) = self.serializer.process_msg(&in_sga)?;
            out_sga.set_id(in_sga.get_id());
            out_sgas.push((out_sga, in_sga.get_addr().clone()));
            contexts.push(header_ctx);
        }

        for i in 0..out_sgas.len() {
            let (cf, _addr) = &mut out_sgas[i];
            let ctx = &contexts[i];
            self.serializer.process_header(ctx, cf)?;
        }

        conn.push_sgas(&out_sgas)
            .wrap_err("Unable to send out sgas in datapath.")?;
        Ok(())
    }

    fn get_histograms(&self) -> Vec<Arc<Mutex<HistogramWrapper>>> {
        Vec::default()
    }
}
