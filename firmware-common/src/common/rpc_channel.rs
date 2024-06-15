use embassy_sync::{blocking_mutex::raw::RawMutex, channel::Channel};

pub struct RpcChannel<M: RawMutex, Q, P> {
    request_channal: Channel<M, Q, 1>,
    response_channal: Channel<M, P, 1>,
}

impl<M: RawMutex, Q, P> RpcChannel<M, Q, P> {
    pub fn new() -> Self {
        RpcChannel {
            request_channal: Channel::new(),
            response_channal: Channel::new(),
        }
    }

    pub fn client(&self) -> RpcChannelClient<M, Q, P> {
        RpcChannelClient { channel: self }
    }

    pub fn server(&self) -> RpcChannelServer<M, Q, P> {
        RpcChannelServer { channel: self }
    }
}

pub struct RpcChannelClient<'a, M: RawMutex, Q, P> {
    channel: &'a RpcChannel<M, Q, P>,
}

impl<'a, M: RawMutex, Q, P> RpcChannelClient<'a, M, Q, P> {
    pub async fn call(&mut self, request: Q) -> P {
        self.channel.request_channal.send(request).await;
        self.channel.response_channal.receive().await
    }
}

pub struct RpcChannelServer<'a, M: RawMutex, Q, P> {
    channel: &'a RpcChannel<M, Q, P>,
}

impl<'a, M: RawMutex, Q, P> RpcChannelServer<'a, M, Q, P> {
    pub async fn get_request(&mut self) -> Q {
        self.channel.request_channal.receive().await
    }

    pub async fn send_response(&mut self, response: P) {
        self.channel.response_channal.send(response).await;
    }
}
