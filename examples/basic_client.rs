use {
    async_trait::async_trait,
    relay_client::{
        Client,
        CloseFrame,
        ConnectionHandler,
        ConnectionOptions,
        Error,
        PublishedMessage,
    },
    relay_rpc::{
        auth::{ed25519_dalek::Keypair, rand, AuthToken},
        domain::{AuthSubject, Topic},
    },
    std::{sync::Arc, time::Duration},
    structopt::StructOpt,
};

#[derive(StructOpt)]
struct Args {
    /// Specify WebSocket address.
    #[structopt(short, long, default_value = "wss://relay.walletconnect.com")]
    address: String,

    /// Specify WalletConnect project ID.
    #[structopt(short, long, default_value = "3cbaa32f8fbf3cdcc87d27ca1fa68069")]
    project_id: String,
}

struct Handler {
    name: &'static str,
}

impl Handler {
    fn new(name: &'static str) -> Self {
        Self { name }
    }
}

#[async_trait]
impl ConnectionHandler for Handler {
    async fn connected(&mut self) {
        println!("[{}] connection open", self.name);
    }

    async fn disconnected(&mut self, frame: Option<CloseFrame<'static>>) {
        println!("[{}] connection closed: frame={frame:?}", self.name);
    }

    async fn message_received(&mut self, message: PublishedMessage) {
        println!(
            "[{}] inbound message: topic={} message={}",
            self.name, message.topic, message.message
        );
    }

    async fn inbound_error(&mut self, error: Error) {
        println!("[{}] inbound error: {error}", self.name);
    }

    async fn outbound_error(&mut self, error: Error) {
        println!("[{}] outbound error: {error}", self.name);
    }
}

fn create_conn_opts(address: &str, project_id: &str) -> ConnectionOptions {
    let key = Keypair::generate(&mut rand::thread_rng());

    let auth = AuthToken::new(AuthSubject::generate())
        .aud(address)
        .ttl(Duration::from_secs(60 * 60))
        .as_jwt(&key)
        .unwrap();

    ConnectionOptions::new(project_id, auth).with_address(address)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    let client1 = Client::new(Handler::new("client1"));
    client1
        .connect(create_conn_opts(&args.address, &args.project_id))
        .await?;

    let client2 = Client::new(Handler::new("client2"));
    client2
        .connect(create_conn_opts(&args.address, &args.project_id))
        .await?;

    let topic = Topic::generate();

    let subscription_id = client1.subscribe(topic.clone()).await?;
    println!("[client1] subscribed: topic={topic} subscription_id={subscription_id}");

    client2
        .publish(
            topic.clone(),
            Arc::from("Hello WalletConnect!"),
            0,
            Duration::from_secs(60),
        )
        .await?;

    println!("[client2] published message with topic: {topic}");

    drop(client1);
    drop(client2);

    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("clients disconnected");

    Ok(())
}
