use yew::prelude::*;
use yew::NodeRef;
use std::sync::Arc;
use bns_core::swarm::Swarm;
use bns_core::types::ice_transport::IceTrickleScheme;
use bns_core::encoder::Encoded;
use web_sys::RtcSdpType;
use bns_core::ecc::SecretKey;
use anyhow::Result;
use anyhow::anyhow;
use web_sys::HtmlInputElement;
use crate::discovery::SwarmConfig;
use crate::web3::Web3Provider;
use wasm_bindgen_futures::spawn_local;

pub struct MainView {
    pub swarm: Arc<Swarm>,
    pub web3: Option<Web3Provider>,
    pub key: SecretKey,
    sdp_input_ref: NodeRef,
    http_input_ref: NodeRef

}

pub enum Msg {
    ConnectPeerViaHTTP(String),
    ConnectPeerViaICE(String),
    None
}

impl MainView {
    pub fn new(cfg: &SwarmConfig) -> Self {
        Self {
            swarm: Arc::new(
                Swarm::new(
                    Arc::clone(&cfg.channel),
                    cfg.stun.to_owned(),
                    cfg.key.address())
            ),
            web3: Web3Provider::new(),
            key: cfg.key,
            sdp_input_ref: NodeRef::default(),
            http_input_ref: NodeRef::default()
        }
    }

    pub async fn trickle_handshake(swarm: Arc<Swarm>, key: SecretKey, url: String) -> Result<String> {
        let client = reqwest_wasm::Client::new();
        let transport = swarm.new_transport().await?;
        let req = transport.get_handshake_info(key, RtcSdpType::Offer).await?;
        log::debug!("req: {:?}", req);
        match client
            .post(&url)
            .body(TryInto::<String>::try_into(req)?)
            .send()
            .await?
            .text()
            .await
        {
            Ok(resp) => {
                log::debug!("get answer and candidate from remote");
                let addr = transport
                    .register_remote_info(String::from_utf8(resp.as_bytes().to_vec())?.try_into()?)
                    .await?;
                swarm.register(addr, Arc::clone(&transport));
                Ok("ok".to_string())
            }
            Err(e) => {
                log::error!("someting wrong {:?}", e);
                anyhow::Result::Err(anyhow!(e))
            }
        }
    }
}

impl Component for MainView {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self::new(&SwarmConfig::default())
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ConnectPeerViaHTTP(url) => {
                let swarm = Arc::clone(&self.swarm);
                let key = self.key.clone();
                spawn_local(async move {
                    match Self::trickle_handshake(swarm, key, url).await {
                        Ok(s) => log::info!("{:?}", s),
                        Err(e) => {
                            log::error!("{:?}", e);
                        }
                    }
                });
                true
            },
            Msg::ConnectPeerViaICE(sdp) => false,
            Msg::None => false
        }
    }

    fn changed(&mut self, _ctx: &Context<Self>) -> bool {
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html!{
            <body>
                <div id="viewport">
                <p>
                <input ref={self.sdp_input_ref.clone()} id="remote_sdp_field" type="text" />
                <button onclick={
                    let input = self.http_input_ref.clone();
                    ctx.link().callback(move |_| {
                        match input.cast::<HtmlInputElement>() {
                            Some(input) => Msg::ConnectPeerViaICE(input.value()),
                            None => Msg::None
                        }
                    })
                }>{"Connect with SDP Swap"}</button>
                </p>
                <p>
                <input ref={self.http_input_ref.clone()}id="remote_http_field" type="text" />
                <button onclick={
                    let input = self.http_input_ref.clone();
                    ctx.link().callback(move |_| {
                        match input.cast::<HtmlInputElement>() {
                            Some(input) => Msg::ConnectPeerViaHTTP(input.value()),
                            None => Msg::None
                        }
                    })
                }>{"Connect To Entry Node"}</button>
                </p>
                </div>
            </body>
        }
    }
}
