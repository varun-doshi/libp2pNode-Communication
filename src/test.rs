// use libp2p::{floodsub::Topic, identity::{self, Keypair}, PeerId};
// use log::info;
// use once_cell::sync::Lazy;
// use serde::{Serialize,Deserialize};
// use tokio::sync::mpsc;

// const STORAGE_FILE_PATH: &str = "./recipes.json";

// type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

// static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
// static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
// static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

// type Recipes=Vec<Recipe>;

// #[derive(Debug, Serialize, Deserialize)]
// struct Recipe {
//     id: usize,
//     name: String,
//     ingredients: String,
//     instructions: String,
//     public: bool,
// }

// #[derive(NetworkBehaviour)]
// struct RecipeBehaviour {
//     floodsub: Floodsub,
//     mdns: TokioMdns,
//     #[behaviour(ignore)]
//     response_sender: mpsc::UnboundedSender<ListResponse>,
// }

// #[derive(Debug, Serialize, Deserialize)]
// enum ListMode {
//     ALL,
//     One(String),
// }

// #[derive(Debug, Serialize, Deserialize)]
// struct ListRequest {
//     mode: ListMode,
// }

// #[derive(Debug, Serialize, Deserialize)]
// struct ListResponse {
//     mode: ListMode,
//     data: Recipes,
//     receiver: String,
// }

// enum EventType {
//     Response(ListResponse),
//     Input(String),
// }

// impl NetworkBehaviourEventProcess<MdnsEvent> for RecipeBehaviour {
//     fn inject_event(&mut self, event: MdnsEvent) {
//         match event {
//             MdnsEvent::Discovered(discovered_list) => {
//                 for (peer, _addr) in discovered_list {
//                     self.floodsub.add_node_to_partial_view(peer);
//                 }
//             }
//             MdnsEvent::Expired(expired_list) => {
//                 for (peer, _addr) in expired_list {
//                     if !self.mdns.has_node(&peer) {
//                         self.floodsub.remove_node_from_partial_view(&peer);
//                     }
//                 }
//             }
//         }
//     }
// }

// impl NetworkBehaviourEventProcess<FloodsubEvent> for RecipeBehaviour {
//     fn inject_event(&mut self, event: FloodsubEvent) {
//         match event {
//             FloodsubEvent::Message(msg) => {
//                 if let Ok(resp) = serde_json::from_slice::<ListResponse>(&msg.data) {
//                     if resp.receiver == PEER_ID.to_string() {
//                         info!("Response from {}:", msg.source);
//                         resp.data.iter().for_each(|r| info!("{:?}", r));
//                     }
//                 } else if let Ok(req) = serde_json::from_slice::<ListRequest>(&msg.data) {
//                     match req.mode {
//                         ListMode::ALL => {
//                             info!("Received ALL req: {:?} from {:?}", req, msg.source);
//                             respond_with_public_recipes(
//                                 self.response_sender.clone(),
//                                 msg.source.to_string(),
//                             );
//                         }
//                         ListMode::One(ref peer_id) => {
//                             if peer_id == &PEER_ID.to_string() {
//                                 info!("Received req: {:?} from {:?}", req, msg.source);
//                                 respond_with_public_recipes(
//                                     self.response_sender.clone(),
//                                     msg.source.to_string(),
//                                 );
//                             }
//                         }
//                     }
//                 }
//             }
//             _ => (),
//         }
//     }
// }


// #[tokio::main]
// async fn main() {
//     pretty_env_logger::init();

//     info!("Peer Id: {}", PEER_ID.clone());
//     let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

//     let auth_keys = Keypair::<X25519Spec>::new()
//         .into_authentic(&KEYS)
//         .expect("can create auth keys");

//         let transp = TokioTcpConfig::new()
//         .upgrade(upgrade::Version::V1)
//         .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
//         .multiplex(mplex::MplexConfig::new())
//         .boxed();


//         let mut behaviour = RecipeBehaviour {
//             floodsub: Floodsub::new(PEER_ID.clone()),
//             mdns: TokioMdns::new().expect("can create mdns"),
//             response_sender,
//         };
    
//         behaviour.floodsub.subscribe(TOPIC.clone());

//         let mut swarm = SwarmBuilder::new(transp, behaviour, PEER_ID.clone())
//         .executor(Box::new(|fut| {
//             tokio::spawn(fut);
//         }))
//         .build();


//         let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

//         loop {
//             let evt = {
//                 tokio::select! {
//                     line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
//                     event = swarm.next() => {
//                         info!("Unhandled Swarm Event: {:?}", event);
//                         None
//                     },
//                     response = response_rcv.recv() => Some(EventType::Response(response.expect("response exists"))),
//                 }
//             };
//             if let Some(event) = evt {
//                 match event {
//                     EventType::Response(resp) => {
//                         let json = serde_json::to_string(&resp).expect("can jsonify response");
//                         swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
//                     }
//                     EventType::Input(line) => match line.as_str() {
//                         "ls p" => handle_list_peers(&mut swarm).await,
//                         cmd if cmd.starts_with("ls r") => handle_list_recipes(cmd, &mut swarm).await,
//                         cmd if cmd.starts_with("create r") => handle_create_recipe(cmd).await,
//                         cmd if cmd.starts_with("publish r") => handle_publish_recipe(cmd).await,
//                         _ => error!("unknown command"),
//                     },
//                 }
//             }
//         }
    

//         Swarm::listen_on(
//             &mut swarm,
//             "/ip4/0.0.0.0/tcp/0"
//                 .parse()
//                 .expect("can get a local socket"),
//         )
//         .expect("swarm can be started");
// }

// async fn handle_list_peers(swarm: &mut Swarm<RecipeBehaviour>) {
//     info!("Discovered Peers:");
//     let nodes = swarm.mdns.discovered_nodes();
//     let mut unique_peers = HashSet::new();
//     for peer in nodes {
//         unique_peers.insert(peer);
//     }
//     unique_peers.iter().for_each(|p| info!("{}", p));
// }

// async fn handle_create_recipe(cmd: &str) {
//     if let Some(rest) = cmd.strip_prefix("create r") {
//         let elements: Vec<&str> = rest.split("|").collect();
//         if elements.len() < 3 {
//             info!("too few arguments - Format: name|ingredients|instructions");
//         } else {
//             let name = elements.get(0).expect("name is there");
//             let ingredients = elements.get(1).expect("ingredients is there");
//             let instructions = elements.get(2).expect("instructions is there");
//             if let Err(e) = create_new_recipe(name, ingredients, instructions).await {
//                 error!("error creating recipe: {}", e);
//             };
//         }
//     }
// }

// async fn handle_publish_recipe(cmd: &str) {
//     if let Some(rest) = cmd.strip_prefix("publish r") {
//         match rest.trim().parse::<usize>() {
//             Ok(id) => {
//                 if let Err(e) = publish_recipe(id).await {
//                     info!("error publishing recipe with id {}, {}", id, e)
//                 } else {
//                     info!("Published Recipe with id: {}", id);
//                 }
//             }
//             Err(e) => error!("invalid id: {}, {}", rest.trim(), e),
//         };
//     }
// }

// async fn create_new_recipe(name: &str, ingredients: &str, instructions: &str) -> Result<()> {
//     let mut local_recipes = read_local_recipes().await?;
//     let new_id = match local_recipes.iter().max_by_key(|r| r.id) {
//         Some(v) => v.id + 1,
//         None => 0,
//     };
//     local_recipes.push(Recipe {
//         id: new_id,
//         name: name.to_owned(),
//         ingredients: ingredients.to_owned(),
//         instructions: instructions.to_owned(),
//         public: false,
//     });
//     write_local_recipes(&local_recipes).await?;

//     info!("Created recipe:");
//     info!("Name: {}", name);
//     info!("Ingredients: {}", ingredients);
//     info!("Instructions:: {}", instructions);

//     Ok(())
// }

// async fn publish_recipe(id: usize) -> Result<()> {
//     let mut local_recipes = read_local_recipes().await?;
//     local_recipes
//         .iter_mut()
//         .filter(|r| r.id == id)
//         .for_each(|r| r.public = true);
//     write_local_recipes(&local_recipes).await?;
//     Ok(())
// }

// async fn read_local_recipes() -> Result<Recipes> {
//     let content = fs::read(STORAGE_FILE_PATH).await?;
//     let result = serde_json::from_slice(&content)?;
//     Ok(result)
// }

// async fn write_local_recipes(recipes: &Recipes) -> Result<()> {
//     let json = serde_json::to_string(&recipes)?;
//     fs::write(STORAGE_FILE_PATH, &json).await?;
//     Ok(())
// }

// async fn handle_list_recipes(cmd: &str, swarm: &mut Swarm<RecipeBehaviour>) {
//     let rest = cmd.strip_prefix("ls r ");
//     match rest {
//         Some("all") => {
//             let req = ListRequest {
//                 mode: ListMode::ALL,
//             };
//             let json = serde_json::to_string(&req).expect("can jsonify request");
//             swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
//         }
//         Some(recipes_peer_id) => {
//             let req = ListRequest {
//                 mode: ListMode::One(recipes_peer_id.to_owned()),
//             };
//             let json = serde_json::to_string(&req).expect("can jsonify request");
//             swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
//         }
//         None => {
//             match read_local_recipes().await {
//                 Ok(v) => {
//                     info!("Local Recipes ({})", v.len());
//                     v.iter().for_each(|r| info!("{:?}", r));
//                 }
//                 Err(e) => error!("error fetching local recipes: {}", e),
//             };
//         }
//     };
// }

// fn respond_with_public_recipes(sender: mpsc::UnboundedSender<ListResponse>, receiver: String) {
//     tokio::spawn(async move {
//         match read_local_recipes().await {
//             Ok(recipes) => {
//                 let resp = ListResponse {
//                     mode: ListMode::ALL,
//                     receiver,
//                     data: recipes.into_iter().filter(|r| r.public).collect(),
//                 };
//                 if let Err(e) = sender.send(resp) {
//                     error!("error sending response via channel, {}", e);
//                 }
//             }
//             Err(e) => error!("error fetching local recipes to answer ALL request, {}", e),
//         }
//     });
// }


use libp2p::{
    core::{transport::MemoryTransport, upgrade}, floodsub::{Floodsub, Topic}, identity::{self, Keypair}, mdns, noise::{self}, swarm::{NetworkBehaviour, Swarm}, tcp::TcpConfig, yamux, NetworkBehaviour, PeerId, Transport
};
use log::{info, error};
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use std::{collections::HashSet};
use tokio::{io::AsyncBufReadExt, sync::mpsc};

const STORAGE_FILE_PATH: &str = "./recipes.json";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<Keypair> = Lazy::new(|| Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

type Recipes = Vec<Recipe>;

#[derive(Debug, Serialize, Deserialize)]
struct Recipe {
    id: usize,
    name: String,
    ingredients: String,
    instructions: String,
    public: bool,
}

#[derive(NetworkBehaviour)]
struct RecipeBehaviour {
    floodsub: Floodsub,
    mdns: mdns::Behaviour<libp2p::mdns::tokio::Tokio>,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<ListResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
enum ListMode {
    ALL,
    One(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ListRequest {
    mode: ListMode,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListResponse {
    mode: ListMode,
    data: Recipes,
    receiver: String,
}

enum EventType {
    Response(ListResponse),
    Input(String),
}

impl RecipeBehaviour {
    fn new(response_sender: mpsc::UnboundedSender<ListResponse>) -> Self {
        let floodsub = Floodsub::new(*PEER_ID);
        let mdns=mdns::tokio::Behaviour::new(mdns::Config::default(), *PEER_ID).unwrap();
        // let mdns = mdns::tokio::Behaviour::new(mdns::Config::default()).expect("can create mdns service");
        Self {
            floodsub,
            mdns,
            response_sender,
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer Id: {}", PEER_ID.clone());

    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

    // let auth_keys = NoiseKeypair::<X25519Spec>::new()
    //     .into_authentic(&*KEYS)
    //     .expect("can create auth keys");

        let id_keys = identity::Keypair::generate_ed25519();
        let noise = noise::Config::new(&id_keys).unwrap();
        let builder = MemoryTransport::default().upgrade(upgrade::Version::V1).authenticate(noise);

    let transport = TcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(builder).into_authenticated())
        .multiplex(yamux::Config::default())
        .boxed();
    noise::Config::new(builder);

    let mut behaviour = RecipeBehaviour::new(response_sender);
    behaviour.floodsub.subscribe(TOPIC.clone());

    let mut swarm =libp2p::swarm::Swarm::new(transport, behaviour, *PEER_ID)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let line = line.expect("can get line").expect("can read line from stdin");
                handle_input_event(line, &mut swarm).await;
            }
            event = swarm.next() => {
                if let Some(event) = event {
                    info!("Unhandled Swarm Event: {:?}", event);
                }
            }
            response = response_rcv.recv() => {
                if let Some(response) = response {
                    handle_response_event(response, &mut swarm).await;
                }
            }
        }
    }
}

async fn handle_input_event(line: String, swarm: &mut Swarm<RecipeBehaviour>) {
    match line.as_str() {
        "ls p" => handle_list_peers(swarm).await,
        cmd if cmd.starts_with("ls r") => handle_list_recipes(cmd, swarm).await,
        cmd if cmd.starts_with("create r") => handle_create_recipe(cmd).await,
        cmd if cmd.starts_with("publish r") => handle_publish_recipe(cmd).await,
        _ => error!("unknown command"),
    }
}

async fn handle_response_event(response: ListResponse, swarm: &mut Swarm<RecipeBehaviour>) {
    let json = serde_json::to_string(&response).expect("can jsonify response");
    swarm.floodsub.publish(TOPIC.clone(), json.as_bytes());
}

async fn handle_list_peers(swarm: &mut Swarm<RecipeBehaviour>) {
    info!("Discovered Peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().for_each(|p| info!("{}", p));
}

// Handle other recipe-related functions here...

