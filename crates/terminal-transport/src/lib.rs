mod client;
mod server;

use std::{future::Future, pin::Pin};

use tokio::sync::{mpsc, oneshot};

use terminal_protocol::{
    OpenSubscriptionRequest, ProtocolError, RequestEnvelope, ResponseEnvelope, SubscriptionEvent,
};

pub use client::{LocalSocketTransportClient, LocalSocketTransportSubscription};
pub use server::{LocalSocketServerHandle, spawn_local_socket_server};

pub type TransportBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait TransportRequestHandler: Send + Sync {
    fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> TransportBoxFuture<'_, Result<ResponseEnvelope, ProtocolError>>;
}

pub trait TransportSubscriptionHandler: Send + Sync {
    fn open_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> TransportBoxFuture<'_, Result<TransportSubscription, ProtocolError>>;
}

#[derive(Debug)]
pub struct TransportSubscription {
    pub subscription_id: terminal_domain::SubscriptionId,
    pub events: mpsc::Receiver<SubscriptionEvent>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl TransportSubscription {
    #[must_use]
    pub fn new(
        subscription_id: terminal_domain::SubscriptionId,
        events: mpsc::Receiver<SubscriptionEvent>,
        cancel_tx: oneshot::Sender<()>,
    ) -> Self {
        Self { subscription_id, events, cancel_tx: Some(cancel_tx) }
    }

    pub fn cancel(&mut self) {
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use futures_util::{SinkExt as _, StreamExt as _};
    use interprocess::local_socket::{tokio::Stream, traits::tokio::Stream as _};
    use terminal_domain::{BackendKind, OperationId, PaneId, SessionId, SubscriptionId};
    use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
    use terminal_projection::TopologySnapshot;
    use terminal_protocol::{
        ListSessionsResponse, LocalSocketAddress, RequestEnvelope, RequestPayload,
        ResponseEnvelope, ResponsePayload, SubscriptionEvent, SubscriptionRequest,
        SubscriptionRequestEnvelope, TransportResponse, decode_json_frame, encode_json_frame,
    };
    use tokio::{
        sync::{mpsc, oneshot},
        time::timeout,
    };
    use tokio_util::codec::{Framed, LengthDelimitedCodec};

    use super::{
        LocalSocketTransportClient, TransportBoxFuture, TransportRequestHandler,
        TransportSubscription, TransportSubscriptionHandler, spawn_local_socket_server,
    };

    #[derive(Clone)]
    struct StubHandler {
        cancel_observed: Option<Arc<AtomicBool>>,
        emit_initial_event: bool,
    }

    impl Default for StubHandler {
        fn default() -> Self {
            Self { cancel_observed: None, emit_initial_event: true }
        }
    }

    impl TransportRequestHandler for StubHandler {
        fn handle_request(
            &self,
            request: RequestEnvelope,
        ) -> TransportBoxFuture<'_, Result<ResponseEnvelope, terminal_protocol::ProtocolError>>
        {
            Box::pin(async move {
                Ok(ResponseEnvelope {
                    operation_id: request.operation_id,
                    payload: ResponsePayload::ListSessions(ListSessionsResponse {
                        sessions: Vec::new(),
                    }),
                })
            })
        }
    }

    impl TransportSubscriptionHandler for StubHandler {
        fn open_subscription(
            &self,
            _request: terminal_protocol::OpenSubscriptionRequest,
        ) -> TransportBoxFuture<'_, Result<TransportSubscription, terminal_protocol::ProtocolError>>
        {
            let cancel_observed = self.cancel_observed.clone();
            let emit_initial_event = self.emit_initial_event;
            Box::pin(async move {
                let subscription_id = SubscriptionId::new();
                let (events_tx, events_rx) = mpsc::channel(8);
                let (cancel_tx, cancel_rx) = oneshot::channel();

                tokio::spawn(async move {
                    if emit_initial_event {
                        let _ = events_tx.send(topology_event()).await;
                    }
                    let _ = cancel_rx.await;
                    if let Some(flag) = cancel_observed {
                        flag.store(true, Ordering::SeqCst);
                    }
                });

                Ok(TransportSubscription::new(subscription_id, events_rx, cancel_tx))
            })
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn request_reply_roundtrips_through_transport_handlers() {
        let address = unique_address("request-reply");
        let server = spawn_local_socket_server(StubHandler::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketTransportClient::new(address);

        let response = client
            .send_request(RequestPayload::ListSessions)
            .await
            .expect("request should succeed");

        match response.payload {
            ResponsePayload::ListSessions(list) => assert!(list.sessions.is_empty()),
            other => panic!("unexpected payload: {other:?}"),
        }

        server.shutdown().await.expect("server should stop cleanly");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn subscription_lane_opens_receives_and_closes_explicitly() {
        let address = unique_address("subscription-close");
        let cancel_observed = Arc::new(AtomicBool::new(false));
        let server = spawn_local_socket_server(
            StubHandler {
                cancel_observed: Some(cancel_observed.clone()),
                emit_initial_event: true,
            },
            address.clone(),
        )
        .expect("server should bind");
        let client = LocalSocketTransportClient::new(address);

        let mut subscription = client
            .open_subscription(terminal_protocol::OpenSubscriptionRequest {
                session_id: SessionId::new(),
                spec: terminal_backend_api::SubscriptionSpec::SessionTopology,
            })
            .await
            .expect("subscription should open");
        let event = subscription
            .recv()
            .await
            .expect("subscription recv should succeed")
            .expect("subscription should yield an initial event");

        match event {
            SubscriptionEvent::TopologySnapshot(snapshot) => {
                assert_eq!(snapshot.backend_kind, BackendKind::Native);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        subscription.close().await.expect("subscription should close cleanly");
        timeout(Duration::from_secs(1), async {
            while !cancel_observed.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cancel should propagate");

        server.shutdown().await.expect("server should stop cleanly");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn wrong_subscription_id_is_rejected() {
        let address = unique_address("wrong-subscription-id");
        let server = spawn_local_socket_server(
            StubHandler { cancel_observed: None, emit_initial_event: false },
            address.clone(),
        )
        .expect("server should bind");
        let request = RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::OpenSubscription(terminal_protocol::OpenSubscriptionRequest {
                session_id: SessionId::new(),
                spec: terminal_backend_api::SubscriptionSpec::SessionTopology,
            }),
        };
        let mut framed = connect_raw(&address).await;
        framed
            .send(encode_json_frame(&request).expect("request should encode"))
            .await
            .expect("open_subscription request should send");
        let frame = framed
            .next()
            .await
            .expect("subscription-open response should arrive")
            .expect("subscription-open response should decode");
        let response = decode_json_frame::<TransportResponse>(&frame)
            .expect("transport response should decode")
            .into_result()
            .expect("subscription-open response should succeed");

        match response.payload {
            ResponsePayload::SubscriptionOpened(_) => {}
            other => panic!("unexpected payload: {other:?}"),
        }

        let wrong_request = SubscriptionRequestEnvelope {
            subscription_id: SubscriptionId::new(),
            request: SubscriptionRequest::Close,
        };
        framed
            .send(encode_json_frame(&wrong_request).expect("close request should encode"))
            .await
            .expect("wrong close request should send");

        let next = timeout(Duration::from_secs(1), framed.next())
            .await
            .expect("server should react to wrong subscription id");
        assert!(matches!(next, None | Some(Err(_))));

        server.shutdown().await.expect("server should stop cleanly");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn close_tolerates_remote_disconnect_after_server_shutdown() {
        let address = unique_address("disconnect-tolerant-close");
        let server = spawn_local_socket_server(StubHandler::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketTransportClient::new(address);
        let mut subscription = client
            .open_subscription(terminal_protocol::OpenSubscriptionRequest {
                session_id: SessionId::new(),
                spec: terminal_backend_api::SubscriptionSpec::SessionTopology,
            })
            .await
            .expect("subscription should open");
        let _ = subscription.recv().await.expect("initial recv should succeed");

        server.shutdown().await.expect("server should stop cleanly");
        subscription.close().await.expect("close should tolerate a disconnected remote");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn server_can_restart_on_the_same_address() {
        let address = unique_address("restart-same-address");
        let client = LocalSocketTransportClient::new(address.clone());

        let first = spawn_local_socket_server(StubHandler::default(), address.clone())
            .expect("server should bind");
        client
            .send_request(RequestPayload::ListSessions)
            .await
            .expect("first request should succeed");
        first.shutdown().await.expect("first server should stop cleanly");

        let second = spawn_local_socket_server(StubHandler::default(), address.clone())
            .expect("server should rebind");
        client
            .send_request(RequestPayload::ListSessions)
            .await
            .expect("second request should succeed");
        second.shutdown().await.expect("second server should stop cleanly");
    }

    fn unique_address(label: &str) -> LocalSocketAddress {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        LocalSocketAddress::from_runtime_slug(format!(
            "terminal-transport-{label}-{}-{nanos}",
            std::process::id()
        ))
    }

    async fn connect_raw(address: &LocalSocketAddress) -> Framed<Stream, LengthDelimitedCodec> {
        let stream = Stream::connect(address.to_name().expect("address should convert"))
            .await
            .expect("raw transport stream should connect");
        Framed::new(stream, LengthDelimitedCodec::new())
    }

    fn topology_event() -> SubscriptionEvent {
        let session_id = SessionId::new();
        let tab_id = terminal_domain::TabId::new();
        let pane_id = PaneId::new();
        SubscriptionEvent::TopologySnapshot(TopologySnapshot {
            session_id,
            backend_kind: BackendKind::Native,
            tabs: vec![TabSnapshot {
                tab_id,
                title: Some("shell".to_string()),
                root: PaneTreeNode::Leaf { pane_id },
                focused_pane: Some(pane_id),
            }],
            focused_tab: Some(tab_id),
        })
    }
}
