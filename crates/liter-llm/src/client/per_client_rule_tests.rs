//! Tests for `ClientConfig::blocked_networks`, on a real client against loopback listeners.
//!
//! Every listener binds 127.0.0.1 or ::1, the two loopback addresses that every runner has.
//! The allowed listener takes ::1 and the blocked one takes 127.0.0.1, so no test needs the
//! rest of 127.0.0.0/8, which only Linux routes to the loopback interface.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use serial_test::serial;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;
use crate::provider::network_rule::NetworkRule;
use crate::provider::outbound_policy::outbound_resolver;
use crate::provider::{OutboundPolicy, set_outbound_policy};

const MODELS: &str = r#"{"object":"list","data":[]}"#;
const BLOCKED: &str = "127.0.0.1/32";
const BLOCKED_HOST: &str = "127.0.0.1";
const BLOCKED_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const ALLOWED_HOST: &str = "::1";
const ALLOWED_IP: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);

/// A listener that counts accepted connections and answers each with `status` and `extra` headers.
/// Every answer closes the connection, so the pool cannot reuse one and hide a second lookup.
struct Listener {
    port: u16,
    accepts: Arc<AtomicUsize>,
}

impl Listener {
    async fn start(ip: &str, port: u16, status: &'static str, extra: &'static str) -> Self {
        let socket = TcpListener::bind((ip, port)).await.expect("bind loopback listener");
        let port = socket.local_addr().expect("listener address").port();
        let accepts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepts);
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = socket.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                let mut buf = vec![0u8; 16 * 1024];
                let _ = stream.read(&mut buf).await;
                let reply = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                     connection: close\r\n{extra}\r\n{MODELS}",
                    MODELS.len()
                );
                let _ = stream.write_all(reply.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });
        Self { port, accepts }
    }

    fn accepts(&self) -> usize {
        self.accepts.load(Ordering::SeqCst)
    }
}

/// Listener A on the allowed address and listener B on the blocked address, on one port.
async fn pair() -> (Listener, Listener) {
    let a = Listener::start(ALLOWED_HOST, 0, "200 OK", "").await;
    let b = Listener::start(BLOCKED_HOST, a.port, "200 OK", "").await;
    (a, b)
}

/// A DNS lookup that answers `answers[call]`, and the last entry once they run out.
struct ScriptedLookup {
    answers: Vec<Vec<IpAddr>>,
    calls: Arc<AtomicUsize>,
}

impl ScriptedLookup {
    fn new(answers: Vec<Vec<IpAddr>>) -> (Arc<Self>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let lookup = Arc::new(Self {
            answers,
            calls: Arc::clone(&calls),
        });
        (lookup, calls)
    }

    fn flip() -> (Arc<Self>, Arc<AtomicUsize>) {
        Self::new(vec![vec![ALLOWED_IP], vec![BLOCKED_IP]])
    }

    fn fixed(answer: Vec<IpAddr>) -> (Arc<Self>, Arc<AtomicUsize>) {
        Self::new(vec![answer])
    }
}

impl Resolve for ScriptedLookup {
    fn resolve(&self, _name: Name) -> Resolving {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let answer = self.answers[call.min(self.answers.len() - 1)].clone();
        Box::pin(async move {
            let addrs: Addrs = Box::new(answer.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addrs)
        })
    }
}

fn config(base_url: String, rule: Option<&[&str]>, dns_cache_ttl: Option<Duration>) -> ClientConfig {
    let mut builder = ClientConfigBuilder::new("test-key").base_url(base_url).max_retries(0);
    if let Some(entries) = rule {
        builder = builder.blocked_networks(entries.iter().map(|e| (*e).to_owned()).collect());
    }
    let mut config = builder.build();
    config.transport.dns_cache_ttl = dns_cache_ttl;
    config
}

fn client(config: ClientConfig, lookup: Option<Arc<ScriptedLookup>>) -> DefaultClient {
    let lookup = lookup.map(|l| l as Arc<dyn Resolve>);
    DefaultClient::new_with_lookup(config, None, lookup).expect("client builds")
}

fn rebind_url(port: u16) -> String {
    format!("http://rebind.test:{port}/v1")
}

fn assert_forbidden<T: std::fmt::Debug>(result: Result<T>, what: &str) {
    match result {
        Err(LiterLlmError::OutboundForbidden { .. }) => {}
        other => panic!("{what}: expected OutboundForbidden, got {other:?}"),
    }
}

#[tokio::test]
#[serial(outbound_policy)]
async fn flip_answer_is_refused_at_connect() {
    let (a, b) = pair().await;
    let (lookup, calls) = ScriptedLookup::flip();
    let client = client(config(rebind_url(a.port), Some(&[BLOCKED]), None), Some(lookup));
    client.list_models().await.expect("first answer is allowed");
    assert_eq!(a.accepts(), 1);
    assert_forbidden(client.list_models().await, "second answer is blocked");
    assert_eq!(b.accepts(), 0, "the blocked listener got no connection");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
#[serial(outbound_policy)]
async fn no_rule_connects_to_the_same_answer() {
    let (a, b) = pair().await;
    let (lookup, _) = ScriptedLookup::flip();
    let client = client(config(rebind_url(a.port), None, None), Some(lookup));
    client.list_models().await.expect("first request");
    client.list_models().await.expect("second request");
    assert_eq!((a.accepts(), b.accepts()), (1, 1));
}

#[tokio::test]
#[serial(outbound_policy)]
async fn every_answer_of_a_lookup_is_checked() {
    let (a, b) = pair().await;
    let (lookup, _) = ScriptedLookup::fixed(vec![ALLOWED_IP, BLOCKED_IP]);
    let client = client(config(rebind_url(a.port), Some(&[BLOCKED]), None), Some(lookup));
    assert_forbidden(client.list_models().await, "one blocked answer of two");
    assert_eq!((a.accepts(), b.accepts()), (0, 0));
}

#[tokio::test]
#[serial(outbound_policy)]
async fn streaming_client_refuses_a_blocked_answer() {
    let (a, b) = pair().await;
    let (lookup, _) = ScriptedLookup::fixed(vec![BLOCKED_IP]);
    let client = client(config(rebind_url(a.port), Some(&[BLOCKED]), None), Some(lookup));
    let req = ChatCompletionRequest {
        model: "gpt-4o".into(),
        ..Default::default()
    };
    assert_forbidden(client.chat_stream(req).await.map(|_| ()), "stream to a blocked answer");
    assert_eq!((a.accepts(), b.accepts()), (0, 0));
}

#[tokio::test]
#[serial(outbound_policy)]
async fn literal_blocked_host_is_refused_before_send() {
    let (a, b) = pair().await;
    for host in [BLOCKED_HOST, "2130706433", "0177.0.0.1", "0x7f000001"] {
        let (lookup, calls) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
        let url = format!("http://{host}:{}/v1", a.port);
        let client = client(config(url, Some(&[BLOCKED]), None), Some(lookup));
        assert_forbidden(client.list_models().await, host);
        assert_eq!(calls.load(Ordering::SeqCst), 0, "{host} needs no lookup");
    }
    assert_eq!(b.accepts(), 0);
    let open = client(config(format!("http://{BLOCKED_HOST}:{}/v1", a.port), None, None), None);
    open.list_models().await.expect("no rule reaches the literal");
    assert_eq!(b.accepts(), 1);
}

#[tokio::test]
#[serial(outbound_policy)]
async fn literal_mapped_host_is_refused_before_send() {
    let (lookup, calls) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
    let client = client(
        config("http://[::ffff:127.0.0.1]:9/v1".into(), Some(&[BLOCKED]), None),
        Some(lookup),
    );
    assert_forbidden(client.list_models().await, "mapped literal");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[serial(outbound_policy)]
async fn multipart_refuses_an_ip_literal() {
    let b = Listener::start(BLOCKED_HOST, 0, "200 OK", "").await;
    let client = client(
        config(format!("http://{BLOCKED_HOST}:{}/v1", b.port), Some(&[BLOCKED]), None),
        None,
    );
    // ~keep A file upload is the request the client sends as multipart; a transcription is sent as JSON.
    let req = CreateFileRequest {
        file: "AAAA".into(),
        ..Default::default()
    };
    assert_forbidden(client.create_file(req).await, "multipart to a blocked literal");
    assert_eq!(b.accepts(), 0);
}

#[tokio::test]
#[serial(outbound_policy)]
async fn process_exemption_keeps_the_rule() {
    let (a, b) = pair().await;
    let (lookup, _) = ScriptedLookup::flip();
    let origin = url::Url::parse(&format!("http://rebind.test:{}", a.port)).expect("origin");
    set_outbound_policy(OutboundPolicy::Allowlist(vec![origin]));
    let client = client(config(rebind_url(a.port), Some(&[BLOCKED]), None), Some(lookup));
    let first = client.list_models().await;
    let second = client.list_models().await;
    set_outbound_policy(OutboundPolicy::Off);
    first.expect("allowed answer");
    assert_forbidden(second, "the exempt host still faces the rule");
    assert_eq!(b.accepts(), 0);
}

#[tokio::test]
#[serial(env, outbound_policy)]
async fn rule_turns_off_the_env_proxy() {
    let a = Listener::start(ALLOWED_HOST, 0, "200 OK", "").await;
    let proxy = Listener::start("127.0.0.1", 0, "200 OK", "").await;
    let previous = std::env::var_os("HTTP_PROXY");
    // SAFETY: serialised by `serial(env)`; no other test thread reads the variable meanwhile.
    unsafe { std::env::set_var("HTTP_PROXY", format!("http://127.0.0.1:{}", proxy.port)) };
    let (lookup, _) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
    let ruled = client(config(rebind_url(a.port), Some(&[BLOCKED]), None), Some(lookup))
        .list_models()
        .await;
    let (lookup, _) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
    let open = client(config(rebind_url(a.port), None, None), Some(lookup))
        .list_models()
        .await;
    // SAFETY: as above.
    unsafe {
        match previous {
            Some(value) => std::env::set_var("HTTP_PROXY", value),
            None => std::env::remove_var("HTTP_PROXY"),
        }
    }
    ruled.expect("ruled client connects directly");
    open.expect("open client uses the proxy");
    assert_eq!(
        (a.accepts(), proxy.accepts()),
        (1, 1),
        "only the client without a rule used the proxy"
    );
}

#[tokio::test]
#[serial(outbound_policy)]
async fn redirect_is_not_followed() {
    let b = Listener::start(BLOCKED_HOST, 0, "200 OK", "").await;
    let location: &'static str = Box::leak(format!("location: http://{BLOCKED_HOST}:{}/\r\n", b.port).into_boxed_str());
    let a = Listener::start(ALLOWED_HOST, 0, "302 Found", location).await;
    let (lookup, _) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
    let client = client(config(rebind_url(a.port), Some(&[BLOCKED]), None), Some(lookup));
    assert!(client.list_models().await.is_err(), "a 302 is returned, not followed");
    assert_eq!((a.accepts(), b.accepts()), (1, 0));
}

#[tokio::test]
#[serial(outbound_policy)]
async fn dns_cache_and_process_check_stay() {
    let (a, _b) = pair().await;
    let (lookup, calls) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
    let ttl = Some(Duration::from_secs(60));
    let client = client(config(rebind_url(a.port), Some(&[BLOCKED]), ttl), Some(lookup));
    client.list_models().await.expect("first request");
    client.list_models().await.expect("second request");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "the DNS cache stays under the rule");

    let rule = Arc::new(NetworkRule::parse(&[BLOCKED.to_owned()]).expect("rule"));
    let (lookup, _) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
    set_outbound_policy(OutboundPolicy::DenyPrivate);
    let resolver = outbound_resolver(None, Some(rule), Some(lookup)).expect("a resolver");
    let result = resolver.resolve("rebind.test".parse().expect("name")).await;
    set_outbound_policy(OutboundPolicy::Off);
    let error = result.err().expect("the process check refuses loopback");
    assert!(matches!(
        error.downcast_ref::<LiterLlmError>(),
        Some(LiterLlmError::OutboundForbidden { .. })
    ));
}

/// lilbee's preset list: the always-blocked networks plus the tunnel prefixes.
const PRESET_LIST: [&str; 10] = [
    "0.0.0.0/8",
    "169.254.0.0/16",
    "224.0.0.0/4",
    "240.0.0.0/4",
    "::/128",
    "fe80::/10",
    "ff00::/8",
    "64:ff9b::/96",
    "64:ff9b:1::/48",
    "2001::/32",
];

/// Oracle: the IPv4 ranges of `PRESET_LIST`, written out as octet tests.
fn v4_blocked(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 0 || (a == 169 && b == 254) || a >= 224
}

fn forms(v4: Ipv4Addr) -> Vec<(&'static str, std::net::Ipv6Addr, bool)> {
    let [a, b, c, d] = v4.octets();
    let hi = u16::from_be_bytes([a, b]);
    let lo = u16::from_be_bytes([c, d]);
    let v6 = |s: [u16; 8]| std::net::Ipv6Addr::from(s);
    vec![
        ("mapped", v4.to_ipv6_mapped(), false),
        ("6to4", v6([0x2002, hi, lo, 0, 0, 0, 0, 1]), false),
        ("siit", v6([0, 0, 0, 0, 0xffff, 0, hi, lo]), false),
        ("compat", v6([0, 0, 0, 0, 0, 0, hi, lo]), false),
        ("nat64_wkp", v6([0x64, 0xff9b, 0, 0, 0, 0, hi, lo]), true),
        ("local_nat64", v6([0x64, 0xff9b, 1, 0, 0, 0, hi, lo]), true),
        (
            "teredo",
            v6([0x2001, 0, 0x4136, 0xe378, 0x8000, 0x63bf, !hi, !lo]),
            true,
        ),
        ("isatap", v6([0x2001, 0xdb8, 0, 0, 0, 0x5efe, hi, lo]), true),
        ("isatap_ul", v6([0x2001, 0xdb8, 0, 0, 0x200, 0x5efe, hi, lo]), true),
    ]
}

#[test]
fn rule_table_matches_the_generated_forms() {
    let rule = NetworkRule::parse(&PRESET_LIST.map(str::to_owned)).expect("preset list");
    let mut samples: Vec<Ipv4Addr> = [
        [169, 254, 169, 254],
        [0, 0, 0, 0],
        [224, 0, 0, 1],
        [255, 255, 255, 255],
        [240, 0, 0, 1],
        [8, 8, 8, 8],
        [1, 1, 1, 1],
        [93, 184, 216, 34],
        [10, 1, 2, 3],
        [192, 168, 1, 1],
    ]
    .map(Ipv4Addr::from)
    .to_vec();
    let mut seed: u32 = 0x4141_0041;
    for _ in 0..40 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        samples.push(Ipv4Addr::from(seed));
    }
    for v4 in samples {
        assert_eq!(rule.refuses(IpAddr::V4(v4)), v4_blocked(v4), "bare {v4}");
        for (name, v6, outright) in forms(v4) {
            let expected = outright || v4_blocked(v4);
            assert_eq!(rule.refuses(IpAddr::V6(v6)), expected, "{name} form {v6} of {v4}");
        }
    }
}

#[test]
fn isatap_is_refused_outright() {
    let empty = NetworkRule::parse(&[]).expect("empty list");
    for form in ["2001:db8::5efe:8.8.8.8", "2001:db8::200:5efe:8.8.8.8"] {
        assert!(empty.refuses(form.parse().expect("address")), "{form}");
    }
    let (lookup, _) = ScriptedLookup::fixed(vec![ALLOWED_IP]);
    let client = client(config(rebind_url(9), None, None), Some(lookup));
    assert!(client.network_rule.is_none(), "no list sets no rule");
}

#[test]
fn bad_cidr_is_refused_with_its_value() {
    let error = DefaultClient::new(
        config("http://rebind.test:9/v1".into(), Some(&["10.0.0.0/33"]), None),
        None,
    )
    .err()
    .expect("a bad entry is refused");
    match error {
        LiterLlmError::BadRequest { message, .. } => assert!(message.contains("10.0.0.0/33"), "{message}"),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}
